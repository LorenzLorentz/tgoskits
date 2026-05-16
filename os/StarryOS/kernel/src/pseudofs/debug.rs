use alloc::sync::Arc;

use super::{DirMaker, DirMapping, SimpleDir, SimpleFs};

/// Create a new debugfs filesystem.
pub fn new_debugfs() -> axfs_ng_vfs::Filesystem {
    // TODO: update fs_type
    SimpleFs::new_with("debug".into(), 0xffff, debugfs_builder)
}

fn debugfs_builder(fs: Arc<SimpleFs>) -> DirMaker {
    let mut root = DirMapping::new();
    let tracing = crate::tracepoint::init_tracing_dir(fs.clone());
    root.add("tracing", tracing);
    SimpleDir::new_maker(fs, Arc::new(root))
}
