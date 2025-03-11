use crate::fs::{FileInfo, FileSystem};
use std::fmt;
use std::sync::Arc;

/// A default implementation of FileSystem that returns errors for all operations
/// Used as a placeholder during deserialization
#[derive(Clone)]
pub struct DefaultFileSystem;

impl Default for Arc<dyn FileSystem> {
    fn default() -> Self {
        Arc::new(DefaultFileSystem)
    }
}

impl fmt::Debug for DefaultFileSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultFileSystem").finish()
    }
}

impl FileSystem for DefaultFileSystem {
    fn read_file(&self, _path: &str) -> Result<Vec<u8>, String> {
        Err("Default file system is not initialized".to_string())
    }

    fn write_file(&self, _path: &str, _content: &[u8]) -> Result<(), String> {
        Err("Default file system is not initialized".to_string())
    }

    fn list_directory(&self, _path: &str) -> Result<Vec<String>, String> {
        Err("Default file system is not initialized".to_string())
    }

    fn create_directory(&self, _path: &str) -> Result<(), String> {
        Err("Default file system is not initialized".to_string())
    }

    fn get_info(&self, _path: &str) -> Result<Option<FileInfo>, String> {
        Err("Default file system is not initialized".to_string())
    }
    
    fn exists(&self, _path: &str) -> Result<bool, String> {
        Err("Default file system is not initialized".to_string())
    }
}