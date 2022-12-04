use clap::Parser;

#[derive(Parser, Debug)]
enum App {
    #[clap(name = "web")]
    Web,
}

fn main() {
    let app: App = App::parse();

    match app {
        App::Web => {
            let p = upsilon_core::alt_exe("upsilon-web");

            let mut cmd = std::process::Command::new(p);

            #[cfg(unix)]
            cmd.exec(); // replace current process with upsilon-web,
                        // execve-style (only available on unix)

            #[cfg(not(unix))]
            {
                let exit_status = cmd
                    .spawn()
                    .expect("failed to execute process")
                    .wait()
                    .expect("failed to execute process");

                if !exit_status.success() {
                    panic!("upsilon-web failed to execute");
                }
            }
        }
    }
}
