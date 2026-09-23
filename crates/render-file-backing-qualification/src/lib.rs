#![doc = "Portable external-file render-backing qualification for the R0B worker data plane."]

use render_buffer_pool::{
    ReadyRenderBuffer, RenderLeaseId, RenderRasterBindingError, RenderRasterDescriptor,
    RenderWriteLease,
};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Qualification-only external backing rooted in a host-owned directory.
///
/// Each lease generation receives a distinct file capability. This is intentionally more
/// conservative than reusing one pathname per logical slot: an old worker retaining an earlier
/// file handle cannot write into the storage selected for a newer lease generation.
///
/// A production mapped backend may reuse mapping objects, but it must provide equivalent stale
/// writer isolation through worker-death fencing, handle revocation, generation-separated
/// mappings, or another mechanism hidden behind the same lease/descriptor contract.
#[derive(Debug)]
pub struct ExternalFileRenderBacking {
    root: PathBuf,
}

impl ExternalFileRenderBacking {
    pub fn new(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn prepare_lease<Scope>(
        &self,
        lease: &RenderWriteLease<Scope>,
    ) -> Result<PreparedExternalRenderLease, ExternalFileRenderBackingError> {
        let capacity_bytes = u64::try_from(lease.capacity_bytes()).map_err(|_| {
            ExternalFileRenderBackingError::HostSizeNotRepresentable {
                bytes: lease.capacity_bytes(),
            }
        })?;
        let path = self.path_for(lease.id());
        let mut file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(ExternalFileRenderBackingError::Io)?;
        file.set_len(capacity_bytes)
            .map_err(ExternalFileRenderBackingError::Io)?;
        clear_prefix(&mut file, lease.requested_bytes())
            .map_err(ExternalFileRenderBackingError::Io)?;
        file.sync_data()
            .map_err(ExternalFileRenderBackingError::Io)?;

        Ok(PreparedExternalRenderLease {
            lease_id: lease.id(),
            capacity_bytes: lease.capacity_bytes(),
            requested_bytes: lease.requested_bytes(),
            path,
        })
    }

    pub fn raster_capability<Scope>(
        &self,
        prepared: &PreparedExternalRenderLease,
        lease: &RenderWriteLease<Scope>,
        descriptor: RenderRasterDescriptor,
    ) -> Result<ExternalRasterCapability, ExternalFileRenderBackingError> {
        prepared.validate_write_lease(lease)?;
        descriptor
            .validate_write_lease(lease)
            .map_err(ExternalFileRenderBackingError::Descriptor)?;

        Ok(ExternalRasterCapability {
            lease_id: lease.id(),
            path: prepared.path.clone(),
            offset_bytes: descriptor.offset_bytes(),
            byte_length: descriptor.byte_length(),
        })
    }

    pub fn read_ready_raster<Scope>(
        &self,
        prepared: &PreparedExternalRenderLease,
        ready: &ReadyRenderBuffer<Scope>,
        descriptor: RenderRasterDescriptor,
    ) -> Result<Vec<u8>, ExternalFileRenderBackingError> {
        prepared.validate_ready_buffer(ready)?;
        descriptor
            .validate_ready_buffer(ready)
            .map_err(ExternalFileRenderBackingError::Descriptor)?;

        let mut file = File::open(&prepared.path).map_err(ExternalFileRenderBackingError::Io)?;
        let metadata_len = file
            .metadata()
            .map_err(ExternalFileRenderBackingError::Io)?
            .len();
        let required_capacity = u64::try_from(prepared.capacity_bytes).map_err(|_| {
            ExternalFileRenderBackingError::HostSizeNotRepresentable {
                bytes: prepared.capacity_bytes,
            }
        })?;
        if metadata_len < required_capacity {
            return Err(ExternalFileRenderBackingError::BackingTruncated {
                actual: metadata_len,
                required: required_capacity,
            });
        }

        let byte_length = usize::try_from(descriptor.byte_length()).map_err(|_| {
            ExternalFileRenderBackingError::ExternalRangeNotRepresentable {
                offset: descriptor.offset_bytes(),
                byte_length: descriptor.byte_length(),
            }
        })?;
        file.seek(SeekFrom::Start(descriptor.offset_bytes()))
            .map_err(ExternalFileRenderBackingError::Io)?;
        let mut bytes = vec![0_u8; byte_length];
        file.read_exact(&mut bytes)
            .map_err(ExternalFileRenderBackingError::Io)?;
        Ok(bytes)
    }

    pub fn remove(
        &self,
        prepared: &PreparedExternalRenderLease,
    ) -> Result<(), ExternalFileRenderBackingError> {
        fs::remove_file(&prepared.path).map_err(ExternalFileRenderBackingError::Io)
    }

    fn path_for(&self, lease_id: RenderLeaseId) -> PathBuf {
        self.root.join(format!(
            "buffer-{}-generation-{}.rgba",
            lease_id.buffer_id().get(),
            lease_id.generation().get()
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedExternalRenderLease {
    lease_id: RenderLeaseId,
    capacity_bytes: usize,
    requested_bytes: usize,
    path: PathBuf,
}

impl PreparedExternalRenderLease {
    #[must_use]
    pub const fn lease_id(&self) -> RenderLeaseId {
        self.lease_id
    }

    #[must_use]
    pub const fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }

    #[must_use]
    pub const fn requested_bytes(&self) -> usize {
        self.requested_bytes
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn validate_write_lease<Scope>(
        &self,
        lease: &RenderWriteLease<Scope>,
    ) -> Result<(), ExternalFileRenderBackingError> {
        if self.lease_id != lease.id() {
            return Err(ExternalFileRenderBackingError::PreparedLeaseMismatch {
                prepared: self.lease_id,
                live: lease.id(),
            });
        }
        if self.capacity_bytes != lease.capacity_bytes() {
            return Err(ExternalFileRenderBackingError::PreparedCapacityMismatch {
                prepared: self.capacity_bytes,
                live: lease.capacity_bytes(),
            });
        }
        if self.requested_bytes != lease.requested_bytes() {
            return Err(ExternalFileRenderBackingError::PreparedRequestMismatch {
                prepared: self.requested_bytes,
                live: lease.requested_bytes(),
            });
        }
        Ok(())
    }

    fn validate_ready_buffer<Scope>(
        &self,
        ready: &ReadyRenderBuffer<Scope>,
    ) -> Result<(), ExternalFileRenderBackingError> {
        if self.lease_id != ready.id() {
            return Err(ExternalFileRenderBackingError::PreparedLeaseMismatch {
                prepared: self.lease_id,
                live: ready.id(),
            });
        }
        if self.capacity_bytes != ready.capacity_bytes() {
            return Err(ExternalFileRenderBackingError::PreparedCapacityMismatch {
                prepared: self.capacity_bytes,
                live: ready.capacity_bytes(),
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalRasterCapability {
    lease_id: RenderLeaseId,
    path: PathBuf,
    offset_bytes: u64,
    byte_length: u64,
}

impl ExternalRasterCapability {
    #[must_use]
    pub const fn lease_id(&self) -> RenderLeaseId {
        self.lease_id
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub const fn offset_bytes(&self) -> u64 {
        self.offset_bytes
    }

    #[must_use]
    pub const fn byte_length(&self) -> u64 {
        self.byte_length
    }
}

#[derive(Debug)]
pub enum ExternalFileRenderBackingError {
    Io(io::Error),
    Descriptor(RenderRasterBindingError),
    HostSizeNotRepresentable {
        bytes: usize,
    },
    PreparedLeaseMismatch {
        prepared: RenderLeaseId,
        live: RenderLeaseId,
    },
    PreparedCapacityMismatch {
        prepared: usize,
        live: usize,
    },
    PreparedRequestMismatch {
        prepared: usize,
        live: usize,
    },
    ExternalRangeNotRepresentable {
        offset: u64,
        byte_length: u64,
    },
    BackingTruncated {
        actual: u64,
        required: u64,
    },
}

impl fmt::Display for ExternalFileRenderBackingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "external render backing I/O failed: {error}"),
            Self::Descriptor(error) => error.fmt(formatter),
            Self::HostSizeNotRepresentable { bytes } => {
                write!(formatter, "render backing size {bytes} is not representable as u64")
            }
            Self::PreparedLeaseMismatch { prepared, live } => write!(
                formatter,
                "prepared external backing belongs to buffer {} generation {}, live lease is buffer {} generation {}",
                prepared.buffer_id().get(),
                prepared.generation().get(),
                live.buffer_id().get(),
                live.generation().get()
            ),
            Self::PreparedCapacityMismatch { prepared, live } => write!(
                formatter,
                "prepared external backing capacity {prepared} differs from live capacity {live}"
            ),
            Self::PreparedRequestMismatch { prepared, live } => write!(
                formatter,
                "prepared external backing request {prepared} differs from live request {live}"
            ),
            Self::ExternalRangeNotRepresentable {
                offset,
                byte_length,
            } => write!(
                formatter,
                "external render range offset={offset} length={byte_length} is not representable on this host"
            ),
            Self::BackingTruncated { actual, required } => write!(
                formatter,
                "external render backing has {actual} bytes, requires at least {required}"
            ),
        }
    }
}

impl std::error::Error for ExternalFileRenderBackingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Descriptor(error) => Some(error),
            Self::HostSizeNotRepresentable { .. }
            | Self::PreparedLeaseMismatch { .. }
            | Self::PreparedCapacityMismatch { .. }
            | Self::PreparedRequestMismatch { .. }
            | Self::ExternalRangeNotRepresentable { .. }
            | Self::BackingTruncated { .. } => None,
        }
    }
}

fn clear_prefix(file: &mut File, bytes: usize) -> io::Result<()> {
    file.seek(SeekFrom::Start(0))?;
    let zeroes = [0_u8; 8_192];
    let mut remaining = bytes;
    while remaining > 0 {
        let count = remaining.min(zeroes.len());
        file.write_all(&zeroes[..count])?;
        remaining -= count;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use render_buffer_pool::{RenderBufferPool, RenderBufferPoolLimits};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

    fn test_root() -> PathBuf {
        let sequence = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "office-render-file-backing-unit-{}-{sequence}",
            std::process::id()
        ))
    }

    #[test]
    fn prepared_generation_file_has_capacity_and_zeroed_requested_prefix() {
        let root = test_root();
        let backing = ExternalFileRenderBacking::new(&root).expect("test backing");
        let limits = RenderBufferPoolLimits::new(4_096, 1, 4_096, 1).expect("limits");
        let mut pool = RenderBufferPool::new(limits);
        let scope = 1_u64;

        let warmup = pool.acquire(scope, 2_048).expect("warmup");
        pool.cancel(warmup.id(), &scope).expect("warmup cancel");
        let lease = pool.acquire(scope, 1_280).expect("reused lease");
        assert_eq!(lease.capacity_bytes(), 2_048);

        let prepared = backing.prepare_lease(&lease).expect("prepared backing");
        let bytes = fs::read(prepared.path()).expect("read prepared file");
        assert_eq!(bytes.len(), 2_048);
        assert!(bytes[..1_280].iter().all(|byte| *byte == 0));

        backing.remove(&prepared).expect("remove backing");
        fs::remove_dir(root).expect("remove test root");
    }
}
