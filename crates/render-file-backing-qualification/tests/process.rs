use render_buffer_pool::{
    RenderBufferPool, RenderBufferPoolLimits, RenderBufferTransitionError, RenderPixelFormat,
    RenderRasterBindingError, RenderRasterDescriptor,
};
use render_file_backing_qualification::ExternalFileRenderBacking;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new(name: &str) -> Self {
        let sequence = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("office-{name}-{}-{sequence}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create test root");
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn limits() -> RenderBufferPoolLimits {
    RenderBufferPoolLimits::new(8_192, 2, 4_096, 2).expect("valid test limits")
}

fn tile_descriptor<Scope>(
    lease: &render_buffer_pool::RenderWriteLease<Scope>,
) -> RenderRasterDescriptor {
    RenderRasterDescriptor::new(
        lease.id().buffer_id().get(),
        lease.id().generation().get(),
        u64::try_from(lease.capacity_bytes()).expect("capacity fits u64"),
        256,
        1_024,
        16,
        16,
        64,
        RenderPixelFormat::Rgba8,
    )
    .expect("test raster descriptor")
}

fn worker_path() -> &'static str {
    env!("CARGO_BIN_EXE_render-file-backing-writer")
}

#[test]
fn child_process_writes_only_validated_raster_region_before_host_publication() {
    let root = TestRoot::new("render-file-child-write");
    let backing = ExternalFileRenderBacking::new(root.path()).expect("external backing");
    let mut pool = RenderBufferPool::new(limits());
    let scope = 10_u64;

    let warmup = pool.acquire(scope, 2_048).expect("warmup lease");
    pool.cancel(warmup.id(), &scope).expect("release warmup");
    let lease = pool.acquire(scope, 1_280).expect("render lease");
    let descriptor = tile_descriptor(&lease);
    let prepared = backing
        .prepare_lease(&lease)
        .expect("prepare external lease");
    let capability = backing
        .raster_capability(&prepared, &lease, descriptor)
        .expect("validated worker capability");

    let output = Command::new(worker_path())
        .arg("write")
        .arg(capability.path())
        .arg(capability.offset_bytes().to_string())
        .arg(capability.byte_length().to_string())
        .arg("165")
        .output()
        .expect("run external render writer");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("worker stdout"),
        "render_file_worker=written\n"
    );

    let on_disk = fs::read(prepared.path()).expect("read worker backing");
    assert!(on_disk[..256].iter().all(|byte| *byte == 0));
    assert!(on_disk[256..1_280].iter().all(|byte| *byte == 165));

    let ready = pool
        .publish(lease.id(), &scope, 1_280)
        .expect("publish completed external render");
    let raster = backing
        .read_ready_raster(&prepared, &ready, descriptor)
        .expect("read published raster");
    assert_eq!(raster, vec![165_u8; 1_024]);

    backing.remove(&prepared).expect("remove external backing");
}

#[test]
fn stale_worker_capability_cannot_corrupt_new_slot_generation() {
    let root = TestRoot::new("render-file-stale-worker");
    let backing = ExternalFileRenderBacking::new(root.path()).expect("external backing");
    let mut pool = RenderBufferPool::new(limits());
    let old_scope = 20_u64;

    let old_lease = pool.acquire(old_scope, 1_280).expect("old lease");
    let old_descriptor = tile_descriptor(&old_lease);
    let old_prepared = backing
        .prepare_lease(&old_lease)
        .expect("old prepared backing");
    let old_capability = backing
        .raster_capability(&old_prepared, &old_lease, old_descriptor)
        .expect("old worker capability");

    let mut child = Command::new(worker_path())
        .arg("hold-rewrite")
        .arg(old_capability.path())
        .arg(old_capability.offset_bytes().to_string())
        .arg(old_capability.byte_length().to_string())
        .arg("17")
        .arg("34")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn stale worker");

    let stdout = child.stdout.take().expect("worker stdout pipe");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).expect("read worker ready");
    assert_eq!(line, "render_file_worker=ready\n");

    let reclaimed = pool.invalidate_scope(&old_scope);
    assert_eq!(reclaimed.leased_reclaimed, 1);

    let new_scope = 21_u64;
    let new_lease = pool.acquire(new_scope, 1_280).expect("replacement lease");
    assert_eq!(new_lease.id().buffer_id(), old_lease.id().buffer_id());
    assert!(new_lease.id().generation() > old_lease.id().generation());

    let new_descriptor = tile_descriptor(&new_lease);
    let new_prepared = backing
        .prepare_lease(&new_lease)
        .expect("new generation backing");
    assert_ne!(old_prepared.path(), new_prepared.path());

    let new_bytes_before = fs::read(new_prepared.path()).expect("read new backing");
    assert!(new_bytes_before[..1_280].iter().all(|byte| *byte == 0));

    child
        .stdin
        .as_mut()
        .expect("worker stdin pipe")
        .write_all(b"x")
        .expect("release stale worker rewrite");
    line.clear();
    reader
        .read_line(&mut line)
        .expect("read stale worker rewrite confirmation");
    assert_eq!(line, "render_file_worker=rewritten\n");
    assert!(child.wait().expect("wait stale worker").success());

    let old_bytes = fs::read(old_prepared.path()).expect("read old backing");
    assert!(old_bytes[256..1_280].iter().all(|byte| *byte == 34));
    let new_bytes_after =
        fs::read(new_prepared.path()).expect("read new backing after stale write");
    assert!(new_bytes_after[..1_280].iter().all(|byte| *byte == 0));

    assert_eq!(
        old_descriptor.validate_write_lease(&new_lease),
        Err(RenderRasterBindingError::LeaseGenerationMismatch {
            declared: old_lease.id().generation().get(),
            actual: new_lease.id().generation(),
        })
    );
    assert_eq!(
        pool.publish(old_lease.id(), &old_scope, 1_280),
        Err(RenderBufferTransitionError::StaleLease {
            buffer_id: old_lease.id().buffer_id(),
            expected_generation: new_lease.id().generation(),
            actual_generation: old_lease.id().generation(),
        })
    );

    backing.remove(&old_prepared).expect("remove old backing");
    backing.remove(&new_prepared).expect("remove new backing");
}
