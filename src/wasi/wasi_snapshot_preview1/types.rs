pub type ExitCode = u32;
pub type Errno = u16;
pub type Fd = u32;
pub type Size = u32;
pub type Filesize = u64;
pub type Timestamp = u64;
pub type DirCookie = u64;

pub const ERRNO_SUCCESS: Errno = 0;
pub const ERRNO_BADF: Errno = 8;
pub const ERRNO_INVAL: Errno = 28;
pub const ERRNO_NOENT: Errno = 44;
pub const ERRNO_NOTDIR: Errno = 54;
pub const ERRNO_NOTCAPABLE: Errno = 76;

pub const CLOCKID_REALTIME: u32 = 0;
pub const CLOCKID_MONOTONIC: u32 = 1;
pub const CLOCKID_PROCESS_CPUTIME_ID: u32 = 2;
pub const CLOCKID_THREAD_CPUTIME_ID: u32 = 3;

pub const SUBSCRIPTION_TYPE_CLOCK: u8 = 0;
pub const EVENT_TYPE_CLOCK: u8 = 0;

pub const SUBSCRIPTION_CLOCK_ABSTIME: u16 = 1;

pub const OFLAGS_CREAT: u16 = 1;
pub const OFLAGS_DIRECTORY: u16 = 2;
pub const OFLAGS_EXCL: u16 = 4;
pub const OFLAGS_TRUNC: u16 = 8;

pub const FDFLAGS_APPEND: u16 = 1;
pub const FDFLAGS_DSYNC: u16 = 2;
pub const FDFLAGS_NONBLOCK: u16 = 4;
pub const FDFLAGS_RSYNC: u16 = 8;
pub const FDFLAGS_SYNC: u16 = 16;
