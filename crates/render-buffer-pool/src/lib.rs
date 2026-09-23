#![doc = "Bounded host-owned render-buffer admission and lease lifecycle."]

use std::collections::BTreeMap;
use std::fmt;

/// Stable host-owned identifier for one reusable render-buffer slot.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RenderBufferId(u32);

impl RenderBufferId {
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Monotonic reuse generation for one render-buffer slot.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RenderLeaseGeneration(u64);

impl RenderLeaseGeneration {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Identity required to complete or release one specific slot lease.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RenderLeaseId {
    buffer_id: RenderBufferId,
    generation: RenderLeaseGeneration,
}

impl RenderLeaseId {
    #[must_use]
    pub const fn buffer_id(self) -> RenderBufferId {
        self.buffer_id
    }

    #[must_use]
    pub const fn generation(self) -> RenderLeaseGeneration {
        self.generation
    }
}

/// Hard host-owned admission limits for one render-buffer pool.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderBufferPoolLimits {
    max_total_bytes: usize,
    max_slots: usize,
    max_slot_bytes: usize,
    max_outstanding_leases_per_scope: usize,
}

impl RenderBufferPoolLimits {
    pub fn new(
        max_total_bytes: usize,
        max_slots: usize,
        max_slot_bytes: usize,
        max_outstanding_leases_per_scope: usize,
    ) -> Result<Self, RenderBufferPoolLimitsError> {
        if max_total_bytes == 0 {
            return Err(RenderBufferPoolLimitsError::ZeroTotalBytes);
        }
        if max_slots == 0 {
            return Err(RenderBufferPoolLimitsError::ZeroSlots);
        }
        if u32::try_from(max_slots).is_err() {
            return Err(RenderBufferPoolLimitsError::TooManySlots);
        }
        if max_slot_bytes == 0 {
            return Err(RenderBufferPoolLimitsError::ZeroSlotBytes);
        }
        if max_slot_bytes > max_total_bytes {
            return Err(RenderBufferPoolLimitsError::SlotExceedsTotalBudget);
        }
        if max_outstanding_leases_per_scope == 0 {
            return Err(RenderBufferPoolLimitsError::ZeroOutstandingLeasesPerScope);
        }
        Ok(Self {
            max_total_bytes,
            max_slots,
            max_slot_bytes,
            max_outstanding_leases_per_scope,
        })
    }

    #[must_use]
    pub const fn max_total_bytes(self) -> usize {
        self.max_total_bytes
    }

    #[must_use]
    pub const fn max_slots(self) -> usize {
        self.max_slots
    }

    #[must_use]
    pub const fn max_slot_bytes(self) -> usize {
        self.max_slot_bytes
    }

    #[must_use]
    pub const fn max_outstanding_leases_per_scope(self) -> usize {
        self.max_outstanding_leases_per_scope
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderBufferPoolLimitsError {
    ZeroTotalBytes,
    ZeroSlots,
    TooManySlots,
    ZeroSlotBytes,
    SlotExceedsTotalBudget,
    ZeroOutstandingLeasesPerScope,
}

impl fmt::Display for RenderBufferPoolLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroTotalBytes => {
                formatter.write_str("render pool total byte budget must be nonzero")
            }
            Self::ZeroSlots => formatter.write_str("render pool slot budget must be nonzero"),
            Self::TooManySlots => {
                formatter.write_str("render pool slot budget exceeds u32 identity space")
            }
            Self::ZeroSlotBytes => {
                formatter.write_str("render pool maximum slot capacity must be nonzero")
            }
            Self::SlotExceedsTotalBudget => {
                formatter.write_str("render pool maximum slot capacity exceeds total byte budget")
            }
            Self::ZeroOutstandingLeasesPerScope => formatter
                .write_str("render pool per-scope outstanding lease budget must be nonzero"),
        }
    }
}

impl std::error::Error for RenderBufferPoolLimitsError {}

/// Public lifecycle state without exposing a caller's authority/scope value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderBufferSlotState {
    Available,
    Leased,
    Ready,
    Retained,
}

/// Host-issued write authority for one buffer slot generation.
///
/// Cloning this value does not extend the underlying lease lifetime. A stale clone is rejected
/// after recycle/reuse by `RenderLeaseGeneration`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderWriteLease<Scope> {
    id: RenderLeaseId,
    scope: Scope,
    capacity_bytes: usize,
    requested_bytes: usize,
}

impl<Scope> RenderWriteLease<Scope> {
    #[must_use]
    pub const fn id(&self) -> RenderLeaseId {
        self.id
    }

    #[must_use]
    pub const fn scope(&self) -> &Scope {
        &self.scope
    }

    #[must_use]
    pub const fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }

    #[must_use]
    pub const fn requested_bytes(&self) -> usize {
        self.requested_bytes
    }
}

/// Host-visible publication token for bytes completed under one valid write lease.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadyRenderBuffer<Scope> {
    id: RenderLeaseId,
    scope: Scope,
    capacity_bytes: usize,
    written_bytes: usize,
}

impl<Scope> ReadyRenderBuffer<Scope> {
    #[must_use]
    pub const fn id(&self) -> RenderLeaseId {
        self.id
    }

    #[must_use]
    pub const fn scope(&self) -> &Scope {
        &self.scope
    }

    #[must_use]
    pub const fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }

    #[must_use]
    pub const fn written_bytes(&self) -> usize {
        self.written_bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum RenderPixelFormat {
    Rgba8 = 1,
    Bgra8 = 2,
}

impl RenderPixelFormat {
    #[must_use]
    pub const fn bytes_per_pixel(self) -> u32 {
        match self {
            Self::Rgba8 | Self::Bgra8 => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnsupportedRenderPixelFormat(pub u8);

impl fmt::Display for UnsupportedRenderPixelFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unsupported render pixel format {}", self.0)
    }
}

impl std::error::Error for UnsupportedRenderPixelFormat {}

impl TryFrom<u8> for RenderPixelFormat {
    type Error = UnsupportedRenderPixelFormat;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Rgba8),
            2 => Ok(Self::Bgra8),
            other => Err(UnsupportedRenderPixelFormat(other)),
        }
    }
}

/// Fixed-width raster metadata suitable for a bounded worker-control descriptor.
///
/// The descriptor deliberately carries primitive fixed-width values rather than host-width
/// offsets or pointers. Construction validates geometry arithmetic and containment inside the
/// declared slot capacity before the descriptor may be bound to a live lease.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderRasterDescriptor {
    buffer_id: u32,
    lease_generation: u64,
    capacity_bytes: u64,
    offset_bytes: u64,
    byte_length: u64,
    width: u32,
    height: u32,
    stride_bytes: u32,
    pixel_format: RenderPixelFormat,
}

impl RenderRasterDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        buffer_id: u32,
        lease_generation: u64,
        capacity_bytes: u64,
        offset_bytes: u64,
        byte_length: u64,
        width: u32,
        height: u32,
        stride_bytes: u32,
        pixel_format: RenderPixelFormat,
    ) -> Result<Self, RenderRasterDescriptorError> {
        if width == 0 {
            return Err(RenderRasterDescriptorError::ZeroWidth);
        }
        if height == 0 {
            return Err(RenderRasterDescriptorError::ZeroHeight);
        }

        let row_bytes = u64::from(width)
            .checked_mul(u64::from(pixel_format.bytes_per_pixel()))
            .ok_or(RenderRasterDescriptorError::GeometryOverflow)?;
        let stride = u64::from(stride_bytes);
        if stride < row_bytes {
            return Err(RenderRasterDescriptorError::StrideTooSmall {
                stride: stride_bytes,
                minimum: row_bytes,
            });
        }

        let minimum_byte_length = u64::from(height - 1)
            .checked_mul(stride)
            .and_then(|prefix| prefix.checked_add(row_bytes))
            .ok_or(RenderRasterDescriptorError::GeometryOverflow)?;
        if byte_length < minimum_byte_length {
            return Err(RenderRasterDescriptorError::ByteLengthTooSmall {
                declared: byte_length,
                minimum: minimum_byte_length,
            });
        }

        let range_end = offset_bytes
            .checked_add(byte_length)
            .ok_or(RenderRasterDescriptorError::RangeOverflow)?;
        if range_end > capacity_bytes {
            return Err(RenderRasterDescriptorError::RangeExceedsCapacity {
                range_end,
                capacity: capacity_bytes,
            });
        }

        Ok(Self {
            buffer_id,
            lease_generation,
            capacity_bytes,
            offset_bytes,
            byte_length,
            width,
            height,
            stride_bytes,
            pixel_format,
        })
    }

    #[must_use]
    pub const fn buffer_id(self) -> u32 {
        self.buffer_id
    }

    #[must_use]
    pub const fn lease_generation(self) -> u64 {
        self.lease_generation
    }

    #[must_use]
    pub const fn capacity_bytes(self) -> u64 {
        self.capacity_bytes
    }

    #[must_use]
    pub const fn offset_bytes(self) -> u64 {
        self.offset_bytes
    }

    #[must_use]
    pub const fn byte_length(self) -> u64 {
        self.byte_length
    }

    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }

    #[must_use]
    pub const fn stride_bytes(self) -> u32 {
        self.stride_bytes
    }

    #[must_use]
    pub const fn pixel_format(self) -> RenderPixelFormat {
        self.pixel_format
    }

    #[must_use]
    pub const fn range_end(self) -> u64 {
        self.offset_bytes + self.byte_length
    }

    pub fn validate_write_lease<Scope>(
        self,
        lease: &RenderWriteLease<Scope>,
    ) -> Result<(), RenderRasterBindingError> {
        self.validate_identity_and_capacity(lease.id(), lease.capacity_bytes())?;
        let range_end = usize::try_from(self.range_end()).map_err(|_| {
            RenderRasterBindingError::RangeExceedsLeaseRequest {
                range_end: self.range_end(),
                requested: lease.requested_bytes(),
            }
        })?;
        if range_end > lease.requested_bytes() {
            return Err(RenderRasterBindingError::RangeExceedsLeaseRequest {
                range_end: self.range_end(),
                requested: lease.requested_bytes(),
            });
        }
        Ok(())
    }

    pub fn validate_ready_buffer<Scope>(
        self,
        ready: &ReadyRenderBuffer<Scope>,
    ) -> Result<(), RenderRasterBindingError> {
        self.validate_identity_and_capacity(ready.id(), ready.capacity_bytes())?;
        let range_end = usize::try_from(self.range_end()).map_err(|_| {
            RenderRasterBindingError::RangeExceedsPublishedBytes {
                range_end: self.range_end(),
                published: ready.written_bytes(),
            }
        })?;
        if range_end > ready.written_bytes() {
            return Err(RenderRasterBindingError::RangeExceedsPublishedBytes {
                range_end: self.range_end(),
                published: ready.written_bytes(),
            });
        }
        Ok(())
    }

    fn validate_identity_and_capacity(
        self,
        lease_id: RenderLeaseId,
        actual_capacity: usize,
    ) -> Result<(), RenderRasterBindingError> {
        if self.buffer_id != lease_id.buffer_id().get() {
            return Err(RenderRasterBindingError::BufferIdMismatch {
                declared: self.buffer_id,
                actual: lease_id.buffer_id(),
            });
        }
        if self.lease_generation != lease_id.generation().get() {
            return Err(RenderRasterBindingError::LeaseGenerationMismatch {
                declared: self.lease_generation,
                actual: lease_id.generation(),
            });
        }
        if usize::try_from(self.capacity_bytes).ok() != Some(actual_capacity) {
            return Err(RenderRasterBindingError::CapacityMismatch {
                declared: self.capacity_bytes,
                actual: actual_capacity,
            });
        }
        Ok(())
    }

    fn host_range(self) -> Result<std::ops::Range<usize>, RenderRasterBindingError> {
        let start = usize::try_from(self.offset_bytes).map_err(|_| {
            RenderRasterBindingError::HostRangeNotRepresentable {
                offset: self.offset_bytes,
                byte_length: self.byte_length,
            }
        })?;
        let end = usize::try_from(self.range_end()).map_err(|_| {
            RenderRasterBindingError::HostRangeNotRepresentable {
                offset: self.offset_bytes,
                byte_length: self.byte_length,
            }
        })?;
        Ok(start..end)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderRasterDescriptorError {
    ZeroWidth,
    ZeroHeight,
    StrideTooSmall { stride: u32, minimum: u64 },
    GeometryOverflow,
    ByteLengthTooSmall { declared: u64, minimum: u64 },
    RangeOverflow,
    RangeExceedsCapacity { range_end: u64, capacity: u64 },
}

impl fmt::Display for RenderRasterDescriptorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroWidth => formatter.write_str("render raster width must be nonzero"),
            Self::ZeroHeight => formatter.write_str("render raster height must be nonzero"),
            Self::StrideTooSmall { stride, minimum } => write!(
                formatter,
                "render raster stride {stride} is smaller than minimum row width {minimum}"
            ),
            Self::GeometryOverflow => {
                formatter.write_str("render raster geometry overflows fixed-width byte arithmetic")
            }
            Self::ByteLengthTooSmall { declared, minimum } => write!(
                formatter,
                "render raster byte length {declared} is smaller than geometry minimum {minimum}"
            ),
            Self::RangeOverflow => {
                formatter.write_str("render raster offset plus byte length overflows u64")
            }
            Self::RangeExceedsCapacity {
                range_end,
                capacity,
            } => write!(
                formatter,
                "render raster range ends at byte {range_end}, exceeding declared capacity {capacity}"
            ),
        }
    }
}

impl std::error::Error for RenderRasterDescriptorError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderRasterBindingError {
    BufferIdMismatch {
        declared: u32,
        actual: RenderBufferId,
    },
    LeaseGenerationMismatch {
        declared: u64,
        actual: RenderLeaseGeneration,
    },
    CapacityMismatch {
        declared: u64,
        actual: usize,
    },
    RangeExceedsLeaseRequest {
        range_end: u64,
        requested: usize,
    },
    RangeExceedsPublishedBytes {
        range_end: u64,
        published: usize,
    },
    HostRangeNotRepresentable {
        offset: u64,
        byte_length: u64,
    },
}

impl fmt::Display for RenderRasterBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferIdMismatch { declared, actual } => write!(
                formatter,
                "render descriptor names buffer {declared}, live lease names {}",
                actual.get()
            ),
            Self::LeaseGenerationMismatch { declared, actual } => write!(
                formatter,
                "render descriptor names lease generation {declared}, live generation is {}",
                actual.get()
            ),
            Self::CapacityMismatch { declared, actual } => write!(
                formatter,
                "render descriptor declares capacity {declared}, live slot capacity is {actual}"
            ),
            Self::RangeExceedsLeaseRequest {
                range_end,
                requested,
            } => write!(
                formatter,
                "render descriptor range ends at byte {range_end}, exceeding lease request {requested}"
            ),
            Self::RangeExceedsPublishedBytes {
                range_end,
                published,
            } => write!(
                formatter,
                "render descriptor range ends at byte {range_end}, exceeding published prefix {published}"
            ),
            Self::HostRangeNotRepresentable {
                offset,
                byte_length,
            } => write!(
                formatter,
                "render descriptor range offset={offset} length={byte_length} is not representable on this host"
            ),
        }
    }
}

impl std::error::Error for RenderRasterBindingError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderBufferAcquireError {
    ZeroByteRequest,
    RequestExceedsSlotLimit { requested: usize, limit: usize },
    ScopeLeaseLimitReached { limit: usize },
    PoolExhausted,
    LeaseGenerationExhausted { buffer_id: RenderBufferId },
}

impl fmt::Display for RenderBufferAcquireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroByteRequest => formatter.write_str("render lease request must be nonzero"),
            Self::RequestExceedsSlotLimit { requested, limit } => write!(
                formatter,
                "render lease requests {requested} bytes, exceeding slot limit {limit}"
            ),
            Self::ScopeLeaseLimitReached { limit } => write!(
                formatter,
                "render scope already owns the maximum {limit} outstanding write leases"
            ),
            Self::PoolExhausted => formatter.write_str("render buffer pool is exhausted"),
            Self::LeaseGenerationExhausted { buffer_id } => write!(
                formatter,
                "render buffer {} exhausted its lease generation space",
                buffer_id.get()
            ),
        }
    }
}

impl std::error::Error for RenderBufferAcquireError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderBufferTransitionError {
    UnknownBuffer(RenderBufferId),
    StaleLease {
        buffer_id: RenderBufferId,
        expected_generation: RenderLeaseGeneration,
        actual_generation: RenderLeaseGeneration,
    },
    WrongState {
        expected: RenderBufferSlotState,
        actual: RenderBufferSlotState,
    },
    WrongScope,
    ZeroByteCompletion,
    CompletionExceedsRequest {
        written: usize,
        requested: usize,
    },
}

impl fmt::Display for RenderBufferTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownBuffer(buffer_id) => {
                write!(formatter, "unknown render buffer {}", buffer_id.get())
            }
            Self::StaleLease {
                buffer_id,
                expected_generation,
                actual_generation,
            } => write!(
                formatter,
                "stale render lease for buffer {}: expected generation {}, got {}",
                buffer_id.get(),
                expected_generation.get(),
                actual_generation.get()
            ),
            Self::WrongState { expected, actual } => write!(
                formatter,
                "render buffer is in state {actual:?}, expected {expected:?}"
            ),
            Self::WrongScope => formatter.write_str("render buffer lease belongs to another scope"),
            Self::ZeroByteCompletion => {
                formatter.write_str("render buffer completion must publish at least one byte")
            }
            Self::CompletionExceedsRequest { written, requested } => write!(
                formatter,
                "render completion publishes {written} bytes, exceeding requested {requested} bytes"
            ),
        }
    }
}

impl std::error::Error for RenderBufferTransitionError {}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RenderBufferPoolStats {
    pub total_capacity_bytes: usize,
    pub slot_count: usize,
    pub available_slots: usize,
    pub leased_slots: usize,
    pub ready_slots: usize,
    pub retained_slots: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RenderScopeInvalidation {
    pub leased_reclaimed: usize,
    pub ready_reclaimed: usize,
    pub retained_reclaimed: usize,
}

impl RenderScopeInvalidation {
    #[must_use]
    pub const fn reclaimed_slots(self) -> usize {
        self.leased_reclaimed + self.ready_reclaimed + self.retained_reclaimed
    }
}

#[derive(Debug)]
enum SlotState<Scope> {
    Available,
    Leased {
        scope: Scope,
        requested_bytes: usize,
    },
    Ready {
        scope: Scope,
        written_bytes: usize,
    },
    Retained {
        scope: Scope,
        written_bytes: usize,
    },
}

impl<Scope> SlotState<Scope> {
    const fn public_state(&self) -> RenderBufferSlotState {
        match self {
            Self::Available => RenderBufferSlotState::Available,
            Self::Leased { .. } => RenderBufferSlotState::Leased,
            Self::Ready { .. } => RenderBufferSlotState::Ready,
            Self::Retained { .. } => RenderBufferSlotState::Retained,
        }
    }
}

#[derive(Debug)]
struct Slot<Scope> {
    id: RenderBufferId,
    generation: RenderLeaseGeneration,
    capacity_bytes: usize,
    state: SlotState<Scope>,
}

/// Host-owned bounded slot/lease state machine for out-of-band render payloads.
///
/// `Scope` is deliberately generic. The application composes this pool with its own product-owned
/// authority (for example `SessionAuthorityStamp`) without making this low-level resource crate
/// depend on session, engine, transport or UI types.
#[derive(Debug)]
pub struct RenderBufferPool<Scope> {
    limits: RenderBufferPoolLimits,
    slots: Vec<Slot<Scope>>,
    total_capacity_bytes: usize,
    outstanding_by_scope: BTreeMap<Scope, usize>,
}

impl<Scope> RenderBufferPool<Scope>
where
    Scope: Clone + Ord,
{
    #[must_use]
    pub fn new(limits: RenderBufferPoolLimits) -> Self {
        Self {
            limits,
            slots: Vec::new(),
            total_capacity_bytes: 0,
            outstanding_by_scope: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn limits(&self) -> RenderBufferPoolLimits {
        self.limits
    }

    pub fn acquire(
        &mut self,
        scope: Scope,
        requested_bytes: usize,
    ) -> Result<RenderWriteLease<Scope>, RenderBufferAcquireError> {
        if requested_bytes == 0 {
            return Err(RenderBufferAcquireError::ZeroByteRequest);
        }
        if requested_bytes > self.limits.max_slot_bytes {
            return Err(RenderBufferAcquireError::RequestExceedsSlotLimit {
                requested: requested_bytes,
                limit: self.limits.max_slot_bytes,
            });
        }

        let outstanding = self.outstanding_by_scope.get(&scope).copied().unwrap_or(0);
        if outstanding >= self.limits.max_outstanding_leases_per_scope {
            return Err(RenderBufferAcquireError::ScopeLeaseLimitReached {
                limit: self.limits.max_outstanding_leases_per_scope,
            });
        }

        let slot_index = self
            .best_reusable_slot(requested_bytes)
            .or_else(|| self.grow_reusable_slot(requested_bytes))
            .or_else(|| self.allocate_slot(requested_bytes))
            .ok_or(RenderBufferAcquireError::PoolExhausted)?;

        let slot = &mut self.slots[slot_index];
        let next_generation = slot
            .generation
            .0
            .checked_add(1)
            .map(RenderLeaseGeneration)
            .ok_or(RenderBufferAcquireError::LeaseGenerationExhausted { buffer_id: slot.id })?;
        slot.generation = next_generation;
        slot.state = SlotState::Leased {
            scope: scope.clone(),
            requested_bytes,
        };
        *self.outstanding_by_scope.entry(scope.clone()).or_insert(0) += 1;

        Ok(RenderWriteLease {
            id: RenderLeaseId {
                buffer_id: slot.id,
                generation: slot.generation,
            },
            scope,
            capacity_bytes: slot.capacity_bytes,
            requested_bytes,
        })
    }

    pub fn publish(
        &mut self,
        lease_id: RenderLeaseId,
        scope: &Scope,
        written_bytes: usize,
    ) -> Result<ReadyRenderBuffer<Scope>, RenderBufferTransitionError> {
        let slot_index = self.validate_generation(lease_id)?;
        let (lease_scope, requested_bytes) = match &self.slots[slot_index].state {
            SlotState::Leased {
                scope,
                requested_bytes,
            } => (scope, *requested_bytes),
            state => {
                return Err(RenderBufferTransitionError::WrongState {
                    expected: RenderBufferSlotState::Leased,
                    actual: state.public_state(),
                });
            }
        };
        if lease_scope != scope {
            return Err(RenderBufferTransitionError::WrongScope);
        }
        if written_bytes == 0 {
            return Err(RenderBufferTransitionError::ZeroByteCompletion);
        }
        if written_bytes > requested_bytes {
            return Err(RenderBufferTransitionError::CompletionExceedsRequest {
                written: written_bytes,
                requested: requested_bytes,
            });
        }

        let published_scope = lease_scope.clone();
        self.decrement_outstanding(&published_scope);
        let slot = &mut self.slots[slot_index];
        slot.state = SlotState::Ready {
            scope: published_scope.clone(),
            written_bytes,
        };
        Ok(ReadyRenderBuffer {
            id: lease_id,
            scope: published_scope,
            capacity_bytes: slot.capacity_bytes,
            written_bytes,
        })
    }

    pub fn retain(
        &mut self,
        lease_id: RenderLeaseId,
        scope: &Scope,
    ) -> Result<(), RenderBufferTransitionError> {
        let slot_index = self.validate_generation(lease_id)?;
        let (ready_scope, written_bytes) = match &self.slots[slot_index].state {
            SlotState::Ready {
                scope,
                written_bytes,
            } => (scope, *written_bytes),
            state => {
                return Err(RenderBufferTransitionError::WrongState {
                    expected: RenderBufferSlotState::Ready,
                    actual: state.public_state(),
                });
            }
        };
        if ready_scope != scope {
            return Err(RenderBufferTransitionError::WrongScope);
        }
        self.slots[slot_index].state = SlotState::Retained {
            scope: ready_scope.clone(),
            written_bytes,
        };
        Ok(())
    }

    pub fn recycle(
        &mut self,
        lease_id: RenderLeaseId,
        scope: &Scope,
    ) -> Result<(), RenderBufferTransitionError> {
        let slot_index = self.validate_generation(lease_id)?;
        let state = &self.slots[slot_index].state;
        let (SlotState::Ready { scope: owner, .. } | SlotState::Retained { scope: owner, .. }) =
            state
        else {
            return Err(RenderBufferTransitionError::WrongState {
                expected: RenderBufferSlotState::Ready,
                actual: state.public_state(),
            });
        };
        if owner != scope {
            return Err(RenderBufferTransitionError::WrongScope);
        }
        self.slots[slot_index].state = SlotState::Available;
        Ok(())
    }

    pub fn cancel(
        &mut self,
        lease_id: RenderLeaseId,
        scope: &Scope,
    ) -> Result<(), RenderBufferTransitionError> {
        let slot_index = self.validate_generation(lease_id)?;
        let owner = match &self.slots[slot_index].state {
            SlotState::Leased { scope, .. } => scope,
            state => {
                return Err(RenderBufferTransitionError::WrongState {
                    expected: RenderBufferSlotState::Leased,
                    actual: state.public_state(),
                });
            }
        };
        if owner != scope {
            return Err(RenderBufferTransitionError::WrongScope);
        }
        let owner = owner.clone();
        self.decrement_outstanding(&owner);
        self.slots[slot_index].state = SlotState::Available;
        Ok(())
    }

    /// Invalidate every slot associated with a dead/replaced caller scope and make its capacity
    /// reusable. Any later completion carrying an old generation is rejected once the slot is
    /// leased again; before reuse it is rejected because the slot is no longer leased.
    pub fn invalidate_scope(&mut self, scope: &Scope) -> RenderScopeInvalidation {
        let mut result = RenderScopeInvalidation::default();
        for slot in &mut self.slots {
            let reclaimed = match &slot.state {
                SlotState::Leased { scope: owner, .. } if owner == scope => {
                    result.leased_reclaimed += 1;
                    true
                }
                SlotState::Ready { scope: owner, .. } if owner == scope => {
                    result.ready_reclaimed += 1;
                    true
                }
                SlotState::Retained { scope: owner, .. } if owner == scope => {
                    result.retained_reclaimed += 1;
                    true
                }
                _ => false,
            };
            if reclaimed {
                slot.state = SlotState::Available;
            }
        }
        self.outstanding_by_scope.remove(scope);
        result
    }

    pub fn validate_write_lease(
        &self,
        lease: &RenderWriteLease<Scope>,
    ) -> Result<(), RenderBufferTransitionError> {
        let slot_index = self.validate_generation(lease.id())?;
        match &self.slots[slot_index].state {
            SlotState::Leased { scope, .. } => {
                if scope != lease.scope() {
                    return Err(RenderBufferTransitionError::WrongScope);
                }
                Ok(())
            }
            state => Err(RenderBufferTransitionError::WrongState {
                expected: RenderBufferSlotState::Leased,
                actual: state.public_state(),
            }),
        }
    }

    pub fn validate_readable_buffer(
        &self,
        ready: &ReadyRenderBuffer<Scope>,
    ) -> Result<(), RenderBufferTransitionError> {
        let slot_index = self.validate_generation(ready.id())?;
        let state = &self.slots[slot_index].state;
        let (SlotState::Ready { scope: owner, .. } | SlotState::Retained { scope: owner, .. }) =
            state
        else {
            return Err(RenderBufferTransitionError::WrongState {
                expected: RenderBufferSlotState::Ready,
                actual: state.public_state(),
            });
        };
        if owner != ready.scope() {
            return Err(RenderBufferTransitionError::WrongScope);
        }
        Ok(())
    }

    #[must_use]
    pub fn slot_state(&self, buffer_id: RenderBufferId) -> Option<RenderBufferSlotState> {
        self.slot(buffer_id).map(|slot| slot.state.public_state())
    }

    #[must_use]
    pub fn slot_capacity_bytes(&self, buffer_id: RenderBufferId) -> Option<usize> {
        self.slot(buffer_id).map(|slot| slot.capacity_bytes)
    }

    #[must_use]
    pub fn slot_written_bytes(&self, buffer_id: RenderBufferId) -> Option<usize> {
        let slot = self.slot(buffer_id)?;
        match &slot.state {
            SlotState::Ready { written_bytes, .. } | SlotState::Retained { written_bytes, .. } => {
                Some(*written_bytes)
            }
            SlotState::Available | SlotState::Leased { .. } => None,
        }
    }

    #[must_use]
    pub fn stats(&self) -> RenderBufferPoolStats {
        let mut stats = RenderBufferPoolStats {
            total_capacity_bytes: self.total_capacity_bytes,
            slot_count: self.slots.len(),
            ..RenderBufferPoolStats::default()
        };
        for slot in &self.slots {
            match slot.state {
                SlotState::Available => stats.available_slots += 1,
                SlotState::Leased { .. } => stats.leased_slots += 1,
                SlotState::Ready { .. } => stats.ready_slots += 1,
                SlotState::Retained { .. } => stats.retained_slots += 1,
            }
        }
        stats
    }

    fn best_reusable_slot(&self, requested_bytes: usize) -> Option<usize> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| {
                matches!(slot.state, SlotState::Available) && slot.capacity_bytes >= requested_bytes
            })
            .min_by_key(|(_, slot)| slot.capacity_bytes)
            .map(|(index, _)| index)
    }

    fn grow_reusable_slot(&mut self, requested_bytes: usize) -> Option<usize> {
        let (index, current_capacity) = self
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| matches!(slot.state, SlotState::Available))
            .filter(|(_, slot)| slot.capacity_bytes < requested_bytes)
            .max_by_key(|(_, slot)| slot.capacity_bytes)
            .map(|(index, slot)| (index, slot.capacity_bytes))?;
        let additional = requested_bytes.checked_sub(current_capacity)?;
        let new_total = self.total_capacity_bytes.checked_add(additional)?;
        if new_total > self.limits.max_total_bytes {
            return None;
        }
        self.slots[index].capacity_bytes = requested_bytes;
        self.total_capacity_bytes = new_total;
        Some(index)
    }

    fn allocate_slot(&mut self, requested_bytes: usize) -> Option<usize> {
        if self.slots.len() >= self.limits.max_slots {
            return None;
        }
        let new_total = self.total_capacity_bytes.checked_add(requested_bytes)?;
        if new_total > self.limits.max_total_bytes {
            return None;
        }
        let id = RenderBufferId(u32::try_from(self.slots.len()).ok()?);
        self.slots.push(Slot {
            id,
            generation: RenderLeaseGeneration(0),
            capacity_bytes: requested_bytes,
            state: SlotState::Available,
        });
        self.total_capacity_bytes = new_total;
        Some(self.slots.len() - 1)
    }

    fn validate_generation(
        &self,
        lease_id: RenderLeaseId,
    ) -> Result<usize, RenderBufferTransitionError> {
        let index = usize::try_from(lease_id.buffer_id.0)
            .map_err(|_| RenderBufferTransitionError::UnknownBuffer(lease_id.buffer_id))?;
        let Some(slot) = self.slots.get(index) else {
            return Err(RenderBufferTransitionError::UnknownBuffer(
                lease_id.buffer_id,
            ));
        };
        if slot.id != lease_id.buffer_id {
            return Err(RenderBufferTransitionError::UnknownBuffer(
                lease_id.buffer_id,
            ));
        }
        if slot.generation != lease_id.generation {
            return Err(RenderBufferTransitionError::StaleLease {
                buffer_id: lease_id.buffer_id,
                expected_generation: slot.generation,
                actual_generation: lease_id.generation,
            });
        }
        Ok(index)
    }

    fn slot(&self, buffer_id: RenderBufferId) -> Option<&Slot<Scope>> {
        let index = usize::try_from(buffer_id.0).ok()?;
        let slot = self.slots.get(index)?;
        (slot.id == buffer_id).then_some(slot)
    }

    fn decrement_outstanding(&mut self, scope: &Scope) {
        let mut remove = false;
        if let Some(outstanding) = self.outstanding_by_scope.get_mut(scope) {
            *outstanding = outstanding.saturating_sub(1);
            remove = *outstanding == 0;
        }
        if remove {
            self.outstanding_by_scope.remove(scope);
        }
    }
}

/// Host-side byte backing for render-buffer slots.
///
/// `prepare_for_write` is the security/correctness boundary before a lease is exposed to a
/// renderer. Implementations must guarantee at least `capacity_bytes` of host-visible storage
/// and clear the first `requested_bytes` bytes so a reused slot cannot expose pixels from an
/// older authority or revision.
///
/// Platform shared-memory implementations may additionally expose worker descriptors through a
/// separate platform adapter; that handle-transfer concern is intentionally not part of this
/// portable host-storage contract.
pub trait RenderBufferBacking {
    type Error;

    fn prepare_for_write(
        &mut self,
        buffer_id: RenderBufferId,
        capacity_bytes: usize,
        requested_bytes: usize,
    ) -> Result<(), Self::Error>;

    fn bytes_mut(&mut self, buffer_id: RenderBufferId) -> Option<&mut [u8]>;

    fn bytes(&self, buffer_id: RenderBufferId) -> Option<&[u8]>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InMemoryRenderBufferBackingError {
    RequestedBytesExceedCapacity {
        requested: usize,
        capacity: usize,
    },
    SlotTableAllocationFailed {
        buffer_id: RenderBufferId,
    },
    ByteAllocationFailed {
        buffer_id: RenderBufferId,
        capacity: usize,
    },
}

impl fmt::Display for InMemoryRenderBufferBackingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequestedBytesExceedCapacity {
                requested,
                capacity,
            } => write!(
                formatter,
                "render backing request {requested} exceeds slot capacity {capacity}"
            ),
            Self::SlotTableAllocationFailed { buffer_id } => write!(
                formatter,
                "could not allocate render backing slot table through buffer {}",
                buffer_id.get()
            ),
            Self::ByteAllocationFailed {
                buffer_id,
                capacity,
            } => write!(
                formatter,
                "could not allocate {capacity} bytes for render buffer {}",
                buffer_id.get()
            ),
        }
    }
}

impl std::error::Error for InMemoryRenderBufferBackingError {}

/// Portable host-owned backing used to qualify allocation/preparation semantics before selecting
/// platform-specific shared mappings.
#[derive(Debug, Default)]
pub struct InMemoryRenderBufferBacking {
    slots: Vec<Vec<u8>>,
}

impl InMemoryRenderBufferBacking {
    #[must_use]
    pub const fn new() -> Self {
        Self { slots: Vec::new() }
    }

    #[must_use]
    pub fn capacity_bytes(&self, buffer_id: RenderBufferId) -> Option<usize> {
        self.slots.get(buffer_id.get() as usize).map(Vec::len)
    }
}

impl RenderBufferBacking for InMemoryRenderBufferBacking {
    type Error = InMemoryRenderBufferBackingError;

    fn prepare_for_write(
        &mut self,
        buffer_id: RenderBufferId,
        capacity_bytes: usize,
        requested_bytes: usize,
    ) -> Result<(), Self::Error> {
        if requested_bytes > capacity_bytes {
            return Err(
                InMemoryRenderBufferBackingError::RequestedBytesExceedCapacity {
                    requested: requested_bytes,
                    capacity: capacity_bytes,
                },
            );
        }

        let slot_index = buffer_id.get() as usize;
        if self.slots.len() <= slot_index {
            let required_slots = slot_index + 1 - self.slots.len();
            self.slots.try_reserve_exact(required_slots).map_err(|_| {
                InMemoryRenderBufferBackingError::SlotTableAllocationFailed { buffer_id }
            })?;
            self.slots.resize_with(slot_index + 1, Vec::new);
        }

        let bytes = &mut self.slots[slot_index];
        if bytes.len() < capacity_bytes {
            let additional = capacity_bytes - bytes.len();
            bytes.try_reserve_exact(additional).map_err(|_| {
                InMemoryRenderBufferBackingError::ByteAllocationFailed {
                    buffer_id,
                    capacity: capacity_bytes,
                }
            })?;
            bytes.resize(capacity_bytes, 0);
        }

        bytes[..requested_bytes].fill(0);
        Ok(())
    }

    fn bytes_mut(&mut self, buffer_id: RenderBufferId) -> Option<&mut [u8]> {
        self.slots
            .get_mut(buffer_id.get() as usize)
            .map(Vec::as_mut_slice)
    }

    fn bytes(&self, buffer_id: RenderBufferId) -> Option<&[u8]> {
        self.slots.get(buffer_id.get() as usize).map(Vec::as_slice)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum BackedRenderBufferAcquireError<BackingError> {
    Pool(RenderBufferAcquireError),
    Backing {
        error: BackingError,
        rollback_error: Option<RenderBufferTransitionError>,
    },
}

impl<BackingError> fmt::Display for BackedRenderBufferAcquireError<BackingError>
where
    BackingError: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(error) => error.fmt(formatter),
            Self::Backing {
                error,
                rollback_error: None,
            } => write!(formatter, "render backing preparation failed: {error}"),
            Self::Backing {
                error,
                rollback_error: Some(rollback_error),
            } => write!(
                formatter,
                "render backing preparation failed ({error}) and lease rollback also failed ({rollback_error})"
            ),
        }
    }
}

impl<BackingError> std::error::Error for BackedRenderBufferAcquireError<BackingError>
where
    BackingError: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Pool(error) => Some(error),
            Self::Backing { error, .. } => Some(error),
        }
    }
}

impl<BackingError> From<RenderBufferAcquireError> for BackedRenderBufferAcquireError<BackingError> {
    fn from(error: RenderBufferAcquireError) -> Self {
        Self::Pool(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderBufferAccessError {
    Transition(RenderBufferTransitionError),
    MissingBacking {
        buffer_id: RenderBufferId,
    },
    BackingTooSmall {
        buffer_id: RenderBufferId,
        available: usize,
        required: usize,
    },
}

impl fmt::Display for RenderBufferAccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transition(error) => error.fmt(formatter),
            Self::MissingBacking { buffer_id } => write!(
                formatter,
                "render buffer {} has no prepared host backing",
                buffer_id.get()
            ),
            Self::BackingTooSmall {
                buffer_id,
                available,
                required,
            } => write!(
                formatter,
                "render buffer {} backing has {available} bytes, requires {required}",
                buffer_id.get()
            ),
        }
    }
}

impl std::error::Error for RenderBufferAccessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transition(error) => Some(error),
            Self::MissingBacking { .. } | Self::BackingTooSmall { .. } => None,
        }
    }
}

impl From<RenderBufferTransitionError> for RenderBufferAccessError {
    fn from(error: RenderBufferTransitionError) -> Self {
        Self::Transition(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderRasterBufferAccessError {
    Buffer(RenderBufferAccessError),
    Descriptor(RenderRasterBindingError),
}

impl fmt::Display for RenderRasterBufferAccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Buffer(error) => error.fmt(formatter),
            Self::Descriptor(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for RenderRasterBufferAccessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Buffer(error) => Some(error),
            Self::Descriptor(error) => Some(error),
        }
    }
}

impl From<RenderBufferAccessError> for RenderRasterBufferAccessError {
    fn from(error: RenderBufferAccessError) -> Self {
        Self::Buffer(error)
    }
}

impl From<RenderRasterBindingError> for RenderRasterBufferAccessError {
    fn from(error: RenderRasterBindingError) -> Self {
        Self::Descriptor(error)
    }
}

/// Composition of the bounded lease state machine with host-owned byte storage.
///
/// This remains a host-side primitive. It deliberately does not decide how a platform mapping
/// handle is transferred to an isolated worker.
#[derive(Debug)]
pub struct BackedRenderBufferPool<Scope, Backing> {
    pool: RenderBufferPool<Scope>,
    backing: Backing,
}

impl<Scope, Backing> BackedRenderBufferPool<Scope, Backing>
where
    Scope: Clone + Ord,
    Backing: RenderBufferBacking,
{
    #[must_use]
    pub fn new(limits: RenderBufferPoolLimits, backing: Backing) -> Self {
        Self {
            pool: RenderBufferPool::new(limits),
            backing,
        }
    }

    pub fn acquire(
        &mut self,
        scope: Scope,
        requested_bytes: usize,
    ) -> Result<RenderWriteLease<Scope>, BackedRenderBufferAcquireError<Backing::Error>> {
        let lease = self.pool.acquire(scope, requested_bytes)?;
        if let Err(error) = self.backing.prepare_for_write(
            lease.id().buffer_id(),
            lease.capacity_bytes(),
            lease.requested_bytes(),
        ) {
            let rollback_error = self.pool.cancel(lease.id(), lease.scope()).err();
            return Err(BackedRenderBufferAcquireError::Backing {
                error,
                rollback_error,
            });
        }
        Ok(lease)
    }

    pub fn write_bytes(
        &mut self,
        lease: &RenderWriteLease<Scope>,
    ) -> Result<&mut [u8], RenderBufferAccessError> {
        self.pool.validate_write_lease(lease)?;
        let buffer_id = lease.id().buffer_id();
        let required = lease.requested_bytes();
        let Some(bytes) = self.backing.bytes_mut(buffer_id) else {
            return Err(RenderBufferAccessError::MissingBacking { buffer_id });
        };
        if bytes.len() < required {
            return Err(RenderBufferAccessError::BackingTooSmall {
                buffer_id,
                available: bytes.len(),
                required,
            });
        }
        Ok(&mut bytes[..required])
    }

    pub fn write_raster_bytes(
        &mut self,
        lease: &RenderWriteLease<Scope>,
        descriptor: RenderRasterDescriptor,
    ) -> Result<&mut [u8], RenderRasterBufferAccessError> {
        descriptor.validate_write_lease(lease)?;
        let range = descriptor.host_range()?;
        let bytes = self.write_bytes(lease)?;
        Ok(&mut bytes[range])
    }

    pub fn publish(
        &mut self,
        lease: &RenderWriteLease<Scope>,
        written_bytes: usize,
    ) -> Result<ReadyRenderBuffer<Scope>, RenderBufferTransitionError> {
        self.pool.publish(lease.id(), lease.scope(), written_bytes)
    }

    pub fn read_bytes(
        &self,
        ready: &ReadyRenderBuffer<Scope>,
    ) -> Result<&[u8], RenderBufferAccessError> {
        self.pool.validate_readable_buffer(ready)?;
        let buffer_id = ready.id().buffer_id();
        let required = ready.written_bytes();
        let Some(bytes) = self.backing.bytes(buffer_id) else {
            return Err(RenderBufferAccessError::MissingBacking { buffer_id });
        };
        if bytes.len() < required {
            return Err(RenderBufferAccessError::BackingTooSmall {
                buffer_id,
                available: bytes.len(),
                required,
            });
        }
        Ok(&bytes[..required])
    }

    pub fn read_raster_bytes(
        &self,
        ready: &ReadyRenderBuffer<Scope>,
        descriptor: RenderRasterDescriptor,
    ) -> Result<&[u8], RenderRasterBufferAccessError> {
        descriptor.validate_ready_buffer(ready)?;
        let range = descriptor.host_range()?;
        let bytes = self.read_bytes(ready)?;
        Ok(&bytes[range])
    }

    pub fn retain(
        &mut self,
        ready: &ReadyRenderBuffer<Scope>,
    ) -> Result<(), RenderBufferTransitionError> {
        self.pool.retain(ready.id(), ready.scope())
    }

    pub fn recycle(
        &mut self,
        ready: &ReadyRenderBuffer<Scope>,
    ) -> Result<(), RenderBufferTransitionError> {
        self.pool.recycle(ready.id(), ready.scope())
    }

    pub fn cancel(
        &mut self,
        lease: &RenderWriteLease<Scope>,
    ) -> Result<(), RenderBufferTransitionError> {
        self.pool.cancel(lease.id(), lease.scope())
    }

    pub fn invalidate_scope(&mut self, scope: &Scope) -> RenderScopeInvalidation {
        self.pool.invalidate_scope(scope)
    }

    #[must_use]
    pub const fn pool(&self) -> &RenderBufferPool<Scope> {
        &self.pool
    }

    #[must_use]
    pub const fn backing(&self) -> &Backing {
        &self.backing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct Scope(u64);

    fn limits(
        max_total_bytes: usize,
        max_slots: usize,
        max_slot_bytes: usize,
        max_outstanding_leases_per_scope: usize,
    ) -> RenderBufferPoolLimits {
        RenderBufferPoolLimits::new(
            max_total_bytes,
            max_slots,
            max_slot_bytes,
            max_outstanding_leases_per_scope,
        )
        .expect("test limits must be valid")
    }

    #[test]
    fn invalid_limits_are_rejected_before_pool_creation() {
        assert_eq!(
            RenderBufferPoolLimits::new(0, 1, 1, 1),
            Err(RenderBufferPoolLimitsError::ZeroTotalBytes)
        );
        assert_eq!(
            RenderBufferPoolLimits::new(1, 0, 1, 1),
            Err(RenderBufferPoolLimitsError::ZeroSlots)
        );
        assert_eq!(
            RenderBufferPoolLimits::new(8, 1, 0, 1),
            Err(RenderBufferPoolLimitsError::ZeroSlotBytes)
        );
        assert_eq!(
            RenderBufferPoolLimits::new(8, 1, 9, 1),
            Err(RenderBufferPoolLimitsError::SlotExceedsTotalBudget)
        );
        assert_eq!(
            RenderBufferPoolLimits::new(8, 1, 8, 0),
            Err(RenderBufferPoolLimitsError::ZeroOutstandingLeasesPerScope)
        );
    }

    #[test]
    fn acquisition_is_bounded_by_request_and_per_scope_limits() {
        let mut pool = RenderBufferPool::new(limits(256, 4, 128, 1));
        let scope = Scope(7);

        assert_eq!(
            pool.acquire(scope, 0),
            Err(RenderBufferAcquireError::ZeroByteRequest)
        );
        assert_eq!(
            pool.acquire(scope, 129),
            Err(RenderBufferAcquireError::RequestExceedsSlotLimit {
                requested: 129,
                limit: 128,
            })
        );

        let first = pool.acquire(scope, 64).expect("first lease must fit");
        assert_eq!(
            pool.acquire(scope, 32),
            Err(RenderBufferAcquireError::ScopeLeaseLimitReached { limit: 1 })
        );

        pool.cancel(first.id(), &scope)
            .expect("cancelling current lease must succeed");
        assert!(pool.acquire(scope, 32).is_ok());
    }

    #[test]
    fn ready_and_retained_lifecycle_recycles_capacity_without_reallocation() {
        let mut pool = RenderBufferPool::new(limits(512, 4, 256, 2));
        let scope = Scope(1);

        let lease = pool.acquire(scope, 128).expect("lease must fit");
        let id = lease.id();
        assert_eq!(
            pool.slot_state(id.buffer_id()),
            Some(RenderBufferSlotState::Leased)
        );

        let ready = pool
            .publish(id, &scope, 96)
            .expect("valid completion must publish");
        assert_eq!(ready.written_bytes(), 96);
        assert_eq!(
            pool.slot_state(id.buffer_id()),
            Some(RenderBufferSlotState::Ready)
        );
        assert_eq!(pool.slot_written_bytes(id.buffer_id()), Some(96));

        pool.retain(id, &scope)
            .expect("ready buffer must be retainable");
        assert_eq!(
            pool.slot_state(id.buffer_id()),
            Some(RenderBufferSlotState::Retained)
        );
        assert_eq!(pool.slot_written_bytes(id.buffer_id()), Some(96));

        pool.recycle(id, &scope)
            .expect("retained buffer must be recyclable");
        assert_eq!(
            pool.slot_state(id.buffer_id()),
            Some(RenderBufferSlotState::Available)
        );
        assert_eq!(pool.slot_written_bytes(id.buffer_id()), None);

        let reused = pool.acquire(scope, 64).expect("capacity must be reusable");
        assert_eq!(reused.id().buffer_id(), id.buffer_id());
        assert!(reused.id().generation() > id.generation());
        assert_eq!(pool.stats().slot_count, 1);
    }

    #[test]
    fn stale_completion_cannot_publish_into_a_reused_slot() {
        let mut pool = RenderBufferPool::new(limits(128, 1, 128, 1));
        let scope = Scope(3);

        let first = pool.acquire(scope, 64).expect("first lease must fit");
        let stale_id = first.id();
        pool.cancel(stale_id, &scope)
            .expect("first lease must cancel");

        let second = pool.acquire(scope, 64).expect("slot must be reusable");
        assert_eq!(second.id().buffer_id(), stale_id.buffer_id());
        assert!(second.id().generation() > stale_id.generation());

        let error = pool
            .publish(stale_id, &scope, 64)
            .expect_err("stale generation must not publish");
        assert_eq!(
            error,
            RenderBufferTransitionError::StaleLease {
                buffer_id: stale_id.buffer_id(),
                expected_generation: second.id().generation(),
                actual_generation: stale_id.generation(),
            }
        );

        pool.publish(second.id(), &scope, 64)
            .expect("current generation must still publish");
    }

    #[test]
    fn wrong_scope_never_completes_or_recycles_another_scope_buffer() {
        let mut pool = RenderBufferPool::new(limits(256, 2, 128, 2));
        let owner = Scope(11);
        let stranger = Scope(12);

        let lease = pool.acquire(owner, 64).expect("lease must fit");
        assert_eq!(
            pool.publish(lease.id(), &stranger, 64),
            Err(RenderBufferTransitionError::WrongScope)
        );

        let ready = pool
            .publish(lease.id(), &owner, 64)
            .expect("owner completion must succeed");
        assert_eq!(
            pool.recycle(ready.id(), &stranger),
            Err(RenderBufferTransitionError::WrongScope)
        );
        pool.recycle(ready.id(), &owner)
            .expect("owner recycle must succeed");
    }

    #[test]
    fn failed_completion_validation_does_not_consume_the_live_lease() {
        let mut pool = RenderBufferPool::new(limits(128, 1, 128, 1));
        let scope = Scope(9);
        let lease = pool.acquire(scope, 80).expect("lease must fit");

        assert_eq!(
            pool.publish(lease.id(), &scope, 0),
            Err(RenderBufferTransitionError::ZeroByteCompletion)
        );
        assert_eq!(
            pool.publish(lease.id(), &scope, 81),
            Err(RenderBufferTransitionError::CompletionExceedsRequest {
                written: 81,
                requested: 80,
            })
        );
        assert_eq!(
            pool.slot_state(lease.id().buffer_id()),
            Some(RenderBufferSlotState::Leased)
        );

        pool.publish(lease.id(), &scope, 80)
            .expect("valid retry must publish");
    }

    #[test]
    fn scope_invalidation_reclaims_leased_ready_and_retained_slots() {
        let mut pool = RenderBufferPool::new(limits(384, 3, 128, 3));
        let scope = Scope(21);

        let leased = pool.acquire(scope, 128).expect("leased slot");
        let ready_lease = pool.acquire(scope, 128).expect("ready slot");
        let retained_lease = pool.acquire(scope, 128).expect("retained slot");

        pool.publish(ready_lease.id(), &scope, 128)
            .expect("ready publication");
        pool.publish(retained_lease.id(), &scope, 128)
            .expect("retained publication");
        pool.retain(retained_lease.id(), &scope)
            .expect("retain must succeed");

        let reclaimed = pool.invalidate_scope(&scope);
        assert_eq!(
            reclaimed,
            RenderScopeInvalidation {
                leased_reclaimed: 1,
                ready_reclaimed: 1,
                retained_reclaimed: 1,
            }
        );
        assert_eq!(reclaimed.reclaimed_slots(), 3);

        let stats = pool.stats();
        assert_eq!(stats.available_slots, 3);
        assert_eq!(stats.leased_slots, 0);
        assert_eq!(stats.ready_slots, 0);
        assert_eq!(stats.retained_slots, 0);

        let replacement = pool
            .acquire(scope, 128)
            .expect("scope budget must be reset after invalidation");
        assert!(replacement.id().generation() > leased.id().generation());
    }

    #[test]
    fn pool_growth_obeys_total_bytes_and_slot_count() {
        let mut pool = RenderBufferPool::new(limits(100, 2, 80, 2));
        let first_scope = Scope(31);
        let second_scope = Scope(32);

        let small = pool.acquire(first_scope, 60).expect("initial slot");
        pool.cancel(small.id(), &first_scope)
            .expect("initial slot cancel");

        let grown = pool.acquire(first_scope, 80).expect("slot can grow");
        assert_eq!(grown.capacity_bytes(), 80);
        assert_eq!(pool.stats().total_capacity_bytes, 80);

        let second = pool.acquire(second_scope, 20).expect("remaining budget");
        assert_eq!(second.capacity_bytes(), 20);
        assert_eq!(pool.stats().total_capacity_bytes, 100);
        assert_eq!(pool.stats().slot_count, 2);

        assert_eq!(
            pool.acquire(Scope(33), 1),
            Err(RenderBufferAcquireError::PoolExhausted)
        );
    }

    fn descriptor_for<Scope>(
        lease: &RenderWriteLease<Scope>,
        offset_bytes: u64,
        byte_length: u64,
        width: u32,
        height: u32,
        stride_bytes: u32,
    ) -> RenderRasterDescriptor {
        RenderRasterDescriptor::new(
            lease.id().buffer_id().get(),
            lease.id().generation().get(),
            u64::try_from(lease.capacity_bytes()).expect("test capacity fits u64"),
            offset_bytes,
            byte_length,
            width,
            height,
            stride_bytes,
            RenderPixelFormat::Rgba8,
        )
        .expect("test descriptor must be structurally valid")
    }

    #[test]
    fn pixel_format_wire_values_are_explicit_and_unknown_values_are_rejected() {
        assert_eq!(RenderPixelFormat::try_from(1), Ok(RenderPixelFormat::Rgba8));
        assert_eq!(RenderPixelFormat::try_from(2), Ok(RenderPixelFormat::Bgra8));
        assert_eq!(
            RenderPixelFormat::try_from(3),
            Err(UnsupportedRenderPixelFormat(3))
        );
    }

    #[test]
    fn raster_descriptor_validates_geometry_and_capacity_before_lease_binding() {
        let descriptor = RenderRasterDescriptor::new(
            7,
            11,
            262_144,
            0,
            262_144,
            256,
            256,
            1_024,
            RenderPixelFormat::Rgba8,
        )
        .expect("qualified tile geometry must be valid");
        assert_eq!(descriptor.range_end(), 262_144);

        assert_eq!(
            RenderRasterDescriptor::new(
                7,
                11,
                262_144,
                0,
                262_144,
                256,
                256,
                1_023,
                RenderPixelFormat::Rgba8,
            ),
            Err(RenderRasterDescriptorError::StrideTooSmall {
                stride: 1_023,
                minimum: 1_024,
            })
        );
        assert_eq!(
            RenderRasterDescriptor::new(
                7,
                11,
                262_144,
                0,
                262_143,
                256,
                256,
                1_024,
                RenderPixelFormat::Rgba8,
            ),
            Err(RenderRasterDescriptorError::ByteLengthTooSmall {
                declared: 262_143,
                minimum: 262_144,
            })
        );
        assert_eq!(
            RenderRasterDescriptor::new(
                7,
                11,
                262_143,
                0,
                262_144,
                256,
                256,
                1_024,
                RenderPixelFormat::Rgba8,
            ),
            Err(RenderRasterDescriptorError::RangeExceedsCapacity {
                range_end: 262_144,
                capacity: 262_143,
            })
        );
    }

    #[test]
    fn raster_descriptor_rejects_fixed_width_range_and_geometry_overflow() {
        assert_eq!(
            RenderRasterDescriptor::new(
                1,
                1,
                u64::MAX,
                u64::MAX - 3,
                4,
                1,
                1,
                4,
                RenderPixelFormat::Rgba8,
            ),
            Err(RenderRasterDescriptorError::RangeOverflow)
        );

        assert_eq!(
            RenderRasterDescriptor::new(
                1,
                1,
                u64::MAX,
                0,
                u64::MAX,
                u32::MAX,
                u32::MAX,
                u32::MAX,
                RenderPixelFormat::Rgba8,
            ),
            Err(RenderRasterDescriptorError::StrideTooSmall {
                stride: u32::MAX,
                minimum: u64::from(u32::MAX) * 4,
            })
        );
    }

    #[test]
    fn raster_descriptor_binds_exactly_to_live_lease_identity_capacity_and_request() {
        let mut pool = RenderBufferPool::new(limits(1_024, 1, 1_024, 1));
        let scope = Scope(46);
        let warmup = pool.acquire(scope, 768).expect("warmup lease");
        pool.cancel(warmup.id(), &scope)
            .expect("warmup lease must release");
        let lease = pool.acquire(scope, 512).expect("smaller reused lease");
        assert_eq!(lease.capacity_bytes(), 768);

        let valid = descriptor_for(&lease, 128, 256, 32, 2, 128);
        valid
            .validate_write_lease(&lease)
            .expect("descriptor must fit current lease");

        let wrong_buffer = RenderRasterDescriptor::new(
            lease.id().buffer_id().get() + 1,
            lease.id().generation().get(),
            u64::try_from(lease.capacity_bytes()).expect("capacity fits"),
            128,
            256,
            32,
            2,
            128,
            RenderPixelFormat::Rgba8,
        )
        .expect("structurally valid");
        assert_eq!(
            wrong_buffer.validate_write_lease(&lease),
            Err(RenderRasterBindingError::BufferIdMismatch {
                declared: lease.id().buffer_id().get() + 1,
                actual: lease.id().buffer_id(),
            })
        );

        let request_escape = descriptor_for(&lease, 384, 128, 16, 2, 64);
        request_escape
            .validate_write_lease(&lease)
            .expect("range ending exactly at requested prefix is valid");

        let beyond_request = RenderRasterDescriptor::new(
            lease.id().buffer_id().get(),
            lease.id().generation().get(),
            u64::try_from(lease.capacity_bytes()).expect("capacity fits"),
            448,
            128,
            16,
            2,
            64,
            RenderPixelFormat::Rgba8,
        )
        .expect("descriptor fits capacity");
        assert_eq!(
            beyond_request.validate_write_lease(&lease),
            Err(RenderRasterBindingError::RangeExceedsLeaseRequest {
                range_end: 576,
                requested: 512,
            })
        );
    }

    #[test]
    fn raster_access_is_sliced_to_validated_region_and_published_prefix() {
        let mut buffers = BackedRenderBufferPool::new(
            limits(1_024, 1, 1_024, 1),
            InMemoryRenderBufferBacking::new(),
        );
        let scope = Scope(47);
        let lease = buffers.acquire(scope, 512).expect("lease");
        let descriptor = descriptor_for(&lease, 128, 256, 32, 2, 128);

        let raster = buffers
            .write_raster_bytes(&lease, descriptor)
            .expect("raster write region");
        assert_eq!(raster.len(), 256);
        raster.fill(0x5a);

        let ready = buffers
            .publish(&lease, 384)
            .expect("publish through range end");
        assert_eq!(
            buffers
                .read_raster_bytes(&ready, descriptor)
                .expect("published raster region"),
            vec![0x5a; 256].as_slice()
        );

        let unpublished_descriptor = RenderRasterDescriptor::new(
            ready.id().buffer_id().get(),
            ready.id().generation().get(),
            u64::try_from(ready.capacity_bytes()).expect("capacity fits"),
            256,
            256,
            32,
            2,
            128,
            RenderPixelFormat::Rgba8,
        )
        .expect("descriptor fits capacity");
        assert_eq!(
            buffers.read_raster_bytes(&ready, unpublished_descriptor),
            Err(RenderRasterBufferAccessError::Descriptor(
                RenderRasterBindingError::RangeExceedsPublishedBytes {
                    range_end: 512,
                    published: 384,
                }
            ))
        );
    }

    #[test]
    fn stale_descriptor_generation_cannot_reopen_reused_backing() {
        let mut buffers =
            BackedRenderBufferPool::new(limits(512, 1, 512, 1), InMemoryRenderBufferBacking::new());
        let scope = Scope(48);

        let first = buffers.acquire(scope, 256).expect("first lease");
        let stale_descriptor = descriptor_for(&first, 0, 256, 32, 2, 128);
        buffers.cancel(&first).expect("cancel first lease");
        let second = buffers.acquire(scope, 256).expect("second lease");

        assert_eq!(
            buffers.write_raster_bytes(&second, stale_descriptor),
            Err(RenderRasterBufferAccessError::Descriptor(
                RenderRasterBindingError::LeaseGenerationMismatch {
                    declared: first.id().generation().get(),
                    actual: second.id().generation(),
                }
            ))
        );
    }

    #[derive(Debug, Default)]
    struct RejectingBacking {
        attempts: usize,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct RejectingBackingError;

    impl fmt::Display for RejectingBackingError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("injected backing preparation failure")
        }
    }

    impl std::error::Error for RejectingBackingError {}

    impl RenderBufferBacking for RejectingBacking {
        type Error = RejectingBackingError;

        fn prepare_for_write(
            &mut self,
            _buffer_id: RenderBufferId,
            _capacity_bytes: usize,
            _requested_bytes: usize,
        ) -> Result<(), Self::Error> {
            self.attempts += 1;
            Err(RejectingBackingError)
        }

        fn bytes_mut(&mut self, _buffer_id: RenderBufferId) -> Option<&mut [u8]> {
            None
        }

        fn bytes(&self, _buffer_id: RenderBufferId) -> Option<&[u8]> {
            None
        }
    }

    #[test]
    fn backed_acquire_prepares_and_clears_reused_bytes() {
        let mut buffers =
            BackedRenderBufferPool::new(limits(128, 1, 128, 1), InMemoryRenderBufferBacking::new());
        let scope = Scope(41);

        let first = buffers.acquire(scope, 64).expect("first lease");
        buffers
            .write_bytes(&first)
            .expect("first writable bytes")
            .fill(0xab);
        let first_ready = buffers.publish(&first, 64).expect("first publication");
        assert!(
            buffers
                .read_bytes(&first_ready)
                .expect("first readable bytes")
                .iter()
                .all(|byte| *byte == 0xab)
        );
        buffers.recycle(&first_ready).expect("first recycle");

        let second = buffers.acquire(scope, 32).expect("reused lease");
        assert_eq!(second.id().buffer_id(), first.id().buffer_id());
        assert!(
            buffers
                .write_bytes(&second)
                .expect("reused writable bytes")
                .iter()
                .all(|byte| *byte == 0)
        );
        assert_eq!(
            buffers.backing().capacity_bytes(second.id().buffer_id()),
            Some(64)
        );
    }

    #[test]
    fn backing_failure_rolls_back_lease_and_scope_budget() {
        let mut buffers =
            BackedRenderBufferPool::new(limits(128, 1, 128, 1), RejectingBacking::default());
        let scope = Scope(42);

        let error = buffers
            .acquire(scope, 64)
            .expect_err("injected backing failure must reject acquisition");
        assert_eq!(
            error,
            BackedRenderBufferAcquireError::Backing {
                error: RejectingBackingError,
                rollback_error: None,
            }
        );
        let stats = buffers.pool().stats();
        assert_eq!(stats.available_slots, 1);
        assert_eq!(stats.leased_slots, 0);

        assert!(matches!(
            buffers.acquire(scope, 64),
            Err(BackedRenderBufferAcquireError::Backing {
                error: RejectingBackingError,
                rollback_error: None,
            })
        ));
        assert_eq!(buffers.backing().attempts, 2);
    }

    #[test]
    fn backed_access_rejects_stale_write_lease_after_slot_reuse() {
        let mut buffers =
            BackedRenderBufferPool::new(limits(128, 1, 128, 1), InMemoryRenderBufferBacking::new());
        let scope = Scope(43);

        let first = buffers.acquire(scope, 64).expect("first lease");
        buffers.cancel(&first).expect("cancel first lease");
        let second = buffers.acquire(scope, 64).expect("second lease");

        let error = buffers
            .write_bytes(&first)
            .expect_err("stale lease must not reopen backing");
        assert_eq!(
            error,
            RenderBufferAccessError::Transition(RenderBufferTransitionError::StaleLease {
                buffer_id: first.id().buffer_id(),
                expected_generation: second.id().generation(),
                actual_generation: first.id().generation(),
            })
        );
    }

    #[test]
    fn backed_ready_view_exposes_only_published_prefix() {
        let mut buffers =
            BackedRenderBufferPool::new(limits(128, 1, 128, 1), InMemoryRenderBufferBacking::new());
        let scope = Scope(44);

        let lease = buffers.acquire(scope, 64).expect("lease");
        let bytes = buffers.write_bytes(&lease).expect("writable bytes");
        bytes[..5].copy_from_slice(b"hello");

        let ready = buffers.publish(&lease, 5).expect("publication");
        assert_eq!(
            buffers.read_bytes(&ready).expect("readable bytes"),
            b"hello"
        );
        buffers.retain(&ready).expect("retain");
        assert_eq!(
            buffers.read_bytes(&ready).expect("retained readable bytes"),
            b"hello"
        );
    }

    #[test]
    fn recycled_ready_token_cannot_read_reused_backing() {
        let mut buffers =
            BackedRenderBufferPool::new(limits(128, 1, 128, 1), InMemoryRenderBufferBacking::new());
        let scope = Scope(45);

        let first = buffers.acquire(scope, 16).expect("first lease");
        buffers
            .write_bytes(&first)
            .expect("first writable bytes")
            .fill(7);
        let first_ready = buffers.publish(&first, 16).expect("first publish");
        buffers.recycle(&first_ready).expect("first recycle");

        let second = buffers.acquire(scope, 16).expect("second lease");
        let error = buffers
            .read_bytes(&first_ready)
            .expect_err("old ready token must be stale after reuse");
        assert_eq!(
            error,
            RenderBufferAccessError::Transition(RenderBufferTransitionError::StaleLease {
                buffer_id: first_ready.id().buffer_id(),
                expected_generation: second.id().generation(),
                actual_generation: first_ready.id().generation(),
            })
        );
    }
}
