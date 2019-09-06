/* This file defines some sanity limits for networking. */
pub const SEND_FILE_SIZE_LIMIT: usize = 2147483647;
pub const SEND_FILE_SIZE_WARNING_LIMIT: usize = 134217728;
pub const MESSAGE_SIZE_LIMIT: usize = 524288;
pub const MESSAGE_SIZE_UNVERSIONED_LIMIT: usize = 201;
pub const MESSAGE_SIZE_WARNING_LIMIT: usize = 65537;
pub const SEND_FILE_CHUNK_SIZE: usize = 65536;
pub const _SEND_VIRTUAL_SIZE_LIMIT: usize = 65536;
pub const _SEND_VIRTUAL_SIZE_WARNING_LIMIT: usize = 8193;
