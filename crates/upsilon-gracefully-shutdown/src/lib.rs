use std::path::Path;
use std::time::Duration;

use tokio::process::{Child, Command};
#[cfg(all(not(target_os = "linux"), not(windows)))]
compile_error!("Unsupported target platform");

fn graceful_shutdown_host_setup(cmd: &mut Command, murderer_file: &Path) {
    let old_cmd = std::mem::replace(
        cmd,
        Command::new(upsilon_core::alt_exe("upsilon-gracefully-shutdown-host")),
    );

    cmd.arg(old_cmd.as_std().get_program());

    cmd.arg("--murderer").arg(murderer_file);

    cmd.args(
        old_cmd
            .as_std()
            .get_args()
            .flat_map(|arg| vec!["--arg".as_ref(), arg]),
    );
}

#[cfg(target_os = "linux")]
pub fn setup_for_graceful_shutdown(cmd: &mut Command, murderer_file: &Path) {
    graceful_shutdown_host_setup(cmd, murderer_file);
}

#[cfg(windows)]
pub fn setup_for_graceful_shutdown(cmd: &mut Command, murderer_file: &Path) {
    graceful_shutdown_host_setup(cmd, murderer_file);

    cmd.creation_flags(
        winapi::um::winbase::CREATE_NEW_PROCESS_GROUP | winapi::um::winbase::NORMAL_PRIORITY_CLASS,
    );
}

fn child_proc_id(child: &Child) -> u32 {
    child.id().expect("Cannot get child process id")
}

async fn grace(child: &mut Child, grace_period: Duration) {
    enum GraceEvent {
        ChildExited,
        Timeout,
    }

    let event = tokio::select! {
        _ = child.wait() => GraceEvent::ChildExited,
        _ = tokio::time::sleep(grace_period) => GraceEvent::Timeout,
    };

    match event {
        GraceEvent::ChildExited => {}
        GraceEvent::Timeout => {
            #[cfg(unix)]
            child.kill().await.expect("Failed to kill child");

            #[cfg(windows)]
            {
                println!("Terminating process");
                let success = unsafe {
                    winapi::um::processthreadsapi::TerminateProcess(
                        child.raw_handle().expect("Cannot get raw handle")
                            as winapi::shared::ntdef::HANDLE,
                        1,
                    ) != 0
                };

                if !success {
                    panic!("Failed to kill child");
                }
            }
        }
    }
}

pub async fn gracefully_shutdown(child: &mut Child, grace_period: Duration) {
    #[cfg(unix)]
    let success =
        unsafe { libc::kill(child_proc_id(child).try_into().unwrap(), libc::SIGINT) == 0 };

    // on windows, the gracefully-shutdown-host will handle this
    #[cfg(windows)]
    let success = true;

    if !success {
        panic!("Failed to send Ctrl+C signal");
    }

    grace(child, grace_period).await;
}
