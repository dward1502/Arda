//! Payload filter; inherited broker stdin is the sole external socket capability.
use anyhow::{bail, Result};
use std::io::{Seek, Write};

fn instructions() -> Result<Vec<libc::sock_filter>> {
    if !cfg!(target_arch = "x86_64") {
        bail!("payload network filter requires qualified x86_64 ABI");
    }
    let statement = |code, k| libc::sock_filter {
        code,
        jt: 0,
        jf: 0,
        k,
    };
    let jump = |k, jt, jf| libc::sock_filter {
        code: 0x15,
        jt,
        jf,
        k,
    };
    let mut filters = vec![
        statement(0x20, 4), // seccomp_data.arch
        jump(0xc000003e, 1, 0),
        statement(0x06, 0x80000000), // kill other syscall ABI
        statement(0x20, 0),
        libc::sock_filter {
            code: 0x45,
            jt: 0,
            jf: 1,
            k: 0x40000000,
        },
        statement(0x06, 0x80000000), // reject x32 ABI
    ];
    for syscall in [
        libc::SYS_socket,
        libc::SYS_connect,
        libc::SYS_accept,
        libc::SYS_accept4,
        libc::SYS_sendmsg,
        libc::SYS_sendmmsg,
        libc::SYS_io_uring_setup,
        libc::SYS_io_uring_enter,
        libc::SYS_io_uring_register,
        libc::SYS_pidfd_getfd,
    ] {
        filters.push(jump(syscall as u32, 0, 1));
        filters.push(statement(0x06, 0x00050000 | libc::EPERM as u32));
    }
    // Internal socketpairs are needed by Python's event loop. Prohibit addressed
    // datagrams so they cannot target a filesystem Unix socket outside the pair.
    filters.push(jump(libc::SYS_sendto as u32, 0, 6));
    filters.push(statement(0x20, 48)); // args[4], low address half
    filters.push(jump(0, 1, 0));
    filters.push(statement(0x06, 0x00050000 | libc::EPERM as u32));
    filters.push(statement(0x20, 52));
    filters.push(jump(0, 1, 0));
    filters.push(statement(0x06, 0x00050000 | libc::EPERM as u32));
    filters.push(statement(0x06, 0x7fff0000));
    Ok(filters)
}

pub fn file() -> Result<std::fs::File> {
    let instructions = instructions()?;
    let mut file = tempfile::tempfile()?;
    for instruction in instructions {
        file.write_all(&instruction.code.to_ne_bytes())?;
        file.write_all(&[instruction.jt, instruction.jf])?;
        file.write_all(&instruction.k.to_ne_bytes())?;
    }
    file.rewind()?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires user/network namespaces and bubblewrap"]
    fn bubblewrap_payload_and_descendant_cannot_connect() {
        use std::os::fd::AsRawFd;
        use std::os::unix::process::CommandExt;
        let file = file().unwrap();
        let fd = file.as_raw_fd();
        let temp = tempfile::tempdir().unwrap();
        let _listener =
            std::os::unix::net::UnixListener::bind(temp.path().join("host.sock")).unwrap();
        let script = r#"
import socket, subprocess, sys, os
for family in (socket.AF_INET, socket.AF_INET6, socket.AF_UNIX):
    for kind in (socket.SOCK_STREAM, socket.SOCK_DGRAM):
        try:
            socket.socket(family, kind)
        except PermissionError:
            pass
        else:
            raise AssertionError('network socket escaped filter')
pair = socket.socketpair()
try:
    pair[0].connect('/fixture/host.sock')
except PermissionError:
    pass
else:
    raise AssertionError('filesystem Unix socket reachable')
assert subprocess.run([sys.executable, '-c', 'import socket; socket.socket()'], capture_output=True).returncode != 0
print('network denied in payload and descendant')
"#;
        let mut command = std::process::Command::new("bwrap");
        command
            .args([
                "--unshare-user",
                "--unshare-net",
                "--ro-bind",
                "/usr",
                "/usr",
                "--symlink",
                "usr/lib64",
                "/lib64",
                "--symlink",
                "usr/lib",
                "/lib",
                "--proc",
                "/proc",
                "--dev",
                "/dev",
                "--ro-bind",
            ])
            .arg(temp.path())
            .arg("/fixture")
            .arg("--seccomp")
            .arg(fd.to_string())
            .args(["--", "/usr/bin/python3", "-I", "-c", script]);
        unsafe {
            command.pre_exec(move || {
                if libc::fcntl(fd, libc::F_SETFD, 0) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("network denied"));
    }
    #[test]
    fn kernel_denies_new_network_and_preserves_internal_channels() {
        let instructions = instructions().unwrap();
        let program = libc::sock_fprog {
            len: instructions.len() as u16,
            filter: instructions.as_ptr() as *mut _,
        };
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0);
        if pid == 0 {
            unsafe {
                if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0
                    || libc::prctl(libc::PR_SET_SECCOMP, 2, &program) != 0
                {
                    libc::_exit(1);
                }
                for family in [libc::AF_INET, libc::AF_INET6, libc::AF_UNIX] {
                    if libc::socket(family, libc::SOCK_STREAM, 0) != -1 {
                        libc::_exit(2);
                    }
                }
                if libc::syscall(libc::SYS_io_uring_setup, 1, std::ptr::null::<u8>()) != -1 {
                    libc::_exit(3);
                }
                let mut pair = [0; 2];
                if libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, pair.as_mut_ptr()) != 0 {
                    libc::_exit(4);
                }
                if libc::send(pair[0], b"x".as_ptr().cast(), 1, 0) != 1 {
                    libc::_exit(5);
                }
                if libc::sendto(pair[0], b"x".as_ptr().cast(), 1, 0, std::ptr::dangling(), 1) != -1
                {
                    libc::_exit(6);
                }
                libc::_exit(0);
            }
        }
        let mut status = 0;
        assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid);
        assert_eq!(status, 0);
    }
}
