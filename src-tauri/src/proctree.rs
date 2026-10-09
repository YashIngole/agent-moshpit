//! Making sure every program the office started dies with it.
//!
//! A terminal closing ends what runs in it on every system, and that is the
//! first line. This is the second, for when the office itself crashes: a job
//! object on Windows that ends its processes when the last handle to it closes.

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

    /// A job whose processes are killed when the last handle to it closes,
    /// which also happens if this app crashes.
    pub struct Tree {
        job: isize,
    }

    // The handle is only ever passed to thread-safe kernel calls.
    unsafe impl Send for Tree {}
    unsafe impl Sync for Tree {}

    impl Tree {
        pub fn new() -> Option<Tree> {
            unsafe {
                let job: HANDLE = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if job.is_null() {
                    return None;
                }
                let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let configured = SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    &limits as *const _ as *const c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                if configured == 0 {
                    CloseHandle(job);
                    return None;
                }
                Some(Tree { job: job as isize })
            }
        }

        /// Tie a program that has just been started, and whatever it starts from now on, to the office.
        pub fn add(&self, pid: u32) {
            unsafe {
                let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
                if process.is_null() {
                    return;
                }
                AssignProcessToJobObject(self.job as HANDLE, process);
                CloseHandle(process);
            }
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.job as HANDLE);
            }
        }
    }
}

#[cfg(unix)]
mod imp {
    /// Nothing to hold: a program started in a pseudo-terminal is hung up on by
    /// the kernel when the terminal's other end closes, and it closes with this app.
    pub struct Tree;

    impl Tree {
        pub fn new() -> Option<Tree> {
            Some(Tree)
        }

        pub fn add(&self, _pid: u32) {}
    }
}

pub use imp::Tree;
