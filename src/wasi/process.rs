use super::types::ExitCode;

pub fn proc_exit(rval: ExitCode) {
    std::process::exit(rval as i32);
}
