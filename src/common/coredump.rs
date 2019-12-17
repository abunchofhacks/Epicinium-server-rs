/* Coredump */

use std::io;

use rlimit::RLIM_INFINITY;

pub fn enable_coredumps() -> io::Result<()>
{
	rlimit::Resource::CORE.set(RLIM_INFINITY, RLIM_INFINITY)
}
