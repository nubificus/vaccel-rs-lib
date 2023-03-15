use anyhow::{bail, Result};
use dns_lookup::lookup_host;
use std::fmt::Write;
use tokio::{fs, process::Command};

pub const VSOCK_PREFIX: &str = "vsock://";
pub const UNIX_PREFIX: &str = "unix://";
pub const TCP_PREFIX: &str = "tcp://";

#[derive(Debug, Clone)]
pub enum State {
    OFF,
    ON { pid: u32 },
}

#[derive(Debug, Clone)]
pub struct Agent {
    agent_path: String,
    endpoint: String,
    debug: String,
    backends: String,
    backends_library: String,
    state: State,
}

impl Agent {
    pub async fn create(
        agent_path: String,
        endpoint: String,
        debug: String,
        backends: String,
        backends_library: String,
    ) -> Agent {
        Agent {
            agent_path,
            endpoint,
            debug,
            backends,
            backends_library,
            state: State::OFF,
        }
    }
    pub async fn start(&mut self) -> Result<()> {
        let mut cmd = Command::new(&self.agent_path);
        //println!("Endpoint: {}",&endpoint);
        cmd.args(["-a", &self.endpoint]);
        cmd.env("VACCEL_DEBUG_LEVEL", &self.debug);
        let mut path = String::default();
        for b in self.backends.split(',') {
            write!(&mut path, "{}libvaccel-{}.so:", &self.backends_library, b);
        }
        path.pop();
        cmd.env("VACCEL_BACKENDS", path);
        let mut child = cmd.spawn()?;
        let pid = match child.id() {
            Some(id) => {
                println!("VACCEL SPAWNED with id: {}", id);
                id
            }
            None => {
                let exit_status = child.wait().await?;
                bail!("VACCEL BAD SPAWN with exit status: {:?}", exit_status);
            }
        };
        self.state = State::ON { pid };
        //child.wait();
        Ok(())
    }
    pub async fn stop(&self) -> Result<()> {
        match self.state {
            State::OFF => println!("Process hasnt started yet"),
            State::ON { pid } => {
                let pid = ::nix::unistd::Pid::from_raw(pid as i32);
                if let Err(err) = ::nix::sys::signal::kill(pid, nix::sys::signal::SIGKILL) {
                    if err != ::nix::Error::ESRCH {
                        bail!("failed to kill virtiofsd pid {} {:?}", pid, err);
                    }
                }
            }
        }
        Ok(())
    }
}
pub async fn construct_vsock(source: String, port: String) -> Result<String> {
    let path = [&source, ":", &port].concat();
    let full_path = [VSOCK_PREFIX, &source, ":", &port].concat();
    match fs::remove_file(&path).await {
        Ok(_) => {
            //    println!("Previous vsock deleted")
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            //    println!("No previous vsock")
        }
        Err(e) => bail!("failed to remove vsock with error: {:?}", e),
    }
    Ok(full_path)
}
pub async fn construct_unix(source: String, port: String) -> Result<String> {
    let path = [&source, "_", &port].concat();
    let full_path = [UNIX_PREFIX, &path].concat();
    match fs::remove_file(&path).await {
        Ok(_) => {
            //    println!("Previous unix socket deleted")
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            //    println!("No previous unix socket")
        }
        Err(e) => bail!("failed to remove unix socket with error: {:?}", e),
    }
    Ok(full_path)
}
pub async fn construct_tcp(arg_source: String, port: String) -> Result<String> {
    let mut source = arg_source;
    let mut dns: Vec<std::net::IpAddr> = vec![];
    //FIXME: better check
    if !source.contains('.') {
        dns = lookup_host(&source).unwrap();
        source = dns[0].to_string();
    }
    let path = [&source, ":", &port].concat();
    let full_path = [TCP_PREFIX, &source, ":", &port].concat();
    match fs::remove_file(&path).await {
        Ok(_) => {
            //    println!("Previous tcp socket deleted")
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            //    println!("No previous tcp socket")
        }
        Err(e) => bail!("failed to remove tcp socket with error: {:?}", e),
    }
    Ok(full_path)
}
