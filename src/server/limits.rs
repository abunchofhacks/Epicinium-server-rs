/* This file defines some sanity limits for networking. */
pub static SEND_FILE_SIZE_LIMIT: usize = 2147483647;
pub static SEND_FILE_SIZE_WARNING_LIMIT: usize = 134217728;
pub static MESSAGE_SIZE_LIMIT: usize = 524288;
pub static MESSAGE_SIZE_UNVERSIONED_LIMIT: usize = 201;
pub static MESSAGE_SIZE_WARNING_LIMIT: usize = 65537;
pub static SEND_FILE_CHUNK_SIZE: usize = 65536;
pub static SEND_VIRTUAL_SIZE_LIMIT: usize = 65536;
pub static SEND_VIRTUAL_SIZE_WARNING_LIMIT: usize = 8193;
