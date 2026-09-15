//! Private, bounded descriptor transfer for ephemeral mount captures.
use std::io;
use std::mem::size_of;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::net::UnixStream;

const MARKER: &[u8] = b"arda.capture.v1";

pub fn parent_pidfd() -> io::Result<OwnedFd> {
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, libc::getpid(), 0u32) };
    if fd == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd as RawFd) })
}

/// Capture's spawning thread stays alive until its helper has been reaped.
/// Linux PIDFD_THREAD is O_EXCL; fail closed on kernels lacking this support.
pub fn spawning_thread_pidfd() -> io::Result<OwnedFd> {
    let fd = unsafe {
        libc::syscall(
            libc::SYS_pidfd_open,
            libc::syscall(libc::SYS_gettid),
            libc::O_EXCL,
        )
    };
    if fd == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd as RawFd) })
}

/// Raw syscalls only: safe to call inside a pre_exec closure. The pidfd is
/// captured before fork, so reparenting/PID reuse cannot validate a new parent.
pub fn arm_parent_death(fd: RawFd) -> io::Result<()> {
    if unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) } == -1 {
        return Err(io::Error::last_os_error());
    }
    let mut state = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    let ready = unsafe { libc::poll(&mut state, 1, 0) };
    if ready != 0 {
        return Err(io::Error::from_raw_os_error(if ready == -1 {
            libc::EINTR
        } else {
            libc::ESRCH
        }));
    }
    Ok(())
}
const MAX_FDS: usize = 4;

pub fn send(socket: &UnixStream, fds: &[RawFd]) -> io::Result<()> {
    if !(3..=MAX_FDS).contains(&fds.len()) {
        return Err(io::Error::other("invalid capture descriptor count"));
    }
    let mut control = [0usize; 16];
    let mut iov = libc::iovec {
        iov_base: MARKER.as_ptr().cast_mut().cast(),
        iov_len: MARKER.len(),
    };
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    message.msg_controllen =
        unsafe { libc::CMSG_SPACE(std::mem::size_of_val(fds) as u32) as usize };
    unsafe {
        let header = libc::CMSG_FIRSTHDR(&message);
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = libc::SCM_RIGHTS;
        (*header).cmsg_len = libc::CMSG_LEN(std::mem::size_of_val(fds) as u32) as usize;
        std::ptr::copy_nonoverlapping(
            fds.as_ptr().cast::<u8>(),
            libc::CMSG_DATA(header),
            std::mem::size_of_val(fds),
        );
        if libc::sendmsg(socket.as_raw_fd(), &message, libc::MSG_NOSIGNAL) != MARKER.len() as isize
        {
            return Err(io::Error::other("capture descriptor transfer failed"));
        }
    }
    Ok(())
}

pub fn receive(socket: &UnixStream, expected: usize) -> io::Result<Vec<OwnedFd>> {
    let mut control = [0usize; 16];
    let mut bytes = [0u8; 32];
    let mut iov = libc::iovec {
        iov_base: bytes.as_mut_ptr().cast(),
        iov_len: bytes.len(),
    };
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    message.msg_controllen = size_of::<[usize; 16]>();
    let count = unsafe { libc::recvmsg(socket.as_raw_fd(), &mut message, libc::MSG_CMSG_CLOEXEC) };
    if count < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut descriptors = Vec::new();
    let mut valid = true;
    unsafe {
        let mut header = libc::CMSG_FIRSTHDR(&message);
        while !header.is_null() {
            let base = libc::CMSG_LEN(0) as usize;
            if (*header).cmsg_level != libc::SOL_SOCKET
                || (*header).cmsg_type != libc::SCM_RIGHTS
                || (*header).cmsg_len < base
            {
                valid = false;
            } else {
                let length = (*header).cmsg_len - base;
                valid &= length.is_multiple_of(size_of::<RawFd>());
                for offset in (0..length).step_by(size_of::<RawFd>()) {
                    if offset + size_of::<RawFd>() > length {
                        break;
                    }
                    let fd = std::ptr::read_unaligned(
                        libc::CMSG_DATA(header).add(offset).cast::<RawFd>(),
                    );
                    descriptors.push(OwnedFd::from_raw_fd(fd));
                }
            }
            header = libc::CMSG_NXTHDR(&message, header);
        }
    }
    if !valid
        || message.msg_flags & (libc::MSG_CTRUNC | libc::MSG_TRUNC) != 0
        || count as usize != MARKER.len()
        || &bytes[..count as usize] != MARKER
        || !(3..=MAX_FDS).contains(&expected)
        || descriptors.len() != expected
    {
        return Err(io::Error::other(
            "invalid capture response or descriptor set",
        ));
    }
    Ok(descriptors)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thread_identity_detects_retirement_while_process_survives() {
        let (thread, process) =
            std::thread::spawn(|| (spawning_thread_pidfd().unwrap(), parent_pidfd().unwrap()))
                .join()
                .unwrap();
        let mut states = [
            libc::pollfd {
                fd: thread.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: process.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        assert_eq!(unsafe { libc::poll(states.as_mut_ptr(), 2, 1000) }, 1);
        assert_ne!(states[0].revents & libc::POLLIN, 0);
        assert_eq!(states[1].revents, 0);
    }

    #[test]
    fn received_capabilities_are_cloexec_and_count_is_exact() {
        let (a, b) = UnixStream::pair().unwrap();
        let file = std::fs::File::open("/").unwrap();
        send(&a, &[file.as_raw_fd(); 3]).unwrap();
        let received = receive(&b, 3).unwrap();
        for fd in received {
            assert_ne!(
                unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
                0
            );
        }
        send(&a, &[file.as_raw_fd(); 3]).unwrap();
        assert!(receive(&b, 4).is_err());
    }
}
