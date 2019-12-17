/* Limits */

use std::io;

use rlimit::RLIM_INFINITY;
pub const MESSAGE_SIZE_LIMIT: usize = 524288;
pub const MESSAGE_SIZE_UNVERSIONED_LIMIT: usize = 201;
pub const MESSAGE_SIZE_WARNING_LIMIT: usize = 65537;

const MAX_SOCKETS: rlimit::rlim = 16384;

pub fn enable_coredumps() -> io::Result<()>
{
	rlimit::Resource::CORE.set(RLIM_INFINITY, RLIM_INFINITY)
}

pub fn increase_sockets() -> io::Result<()>
{
	rlimit::Resource::NOFILE.set(MAX_SOCKETS, MAX_SOCKETS)
}
