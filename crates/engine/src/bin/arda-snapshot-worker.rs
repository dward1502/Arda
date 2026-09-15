//! Retains one private mount tree independently of an Arda client connection.
//! This worker is not yet wired into objective admission or installed services.
use anyhow::{bail, Context, Result};
use arda_engine::objectives::snapshot_protocol::{
    Manifest, Request, MAX_OUTPUT_BYTES, MAX_REQUEST_BYTES, MAX_TIMEOUT_MS,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

mod snapshot_child;
mod snapshot_ephemeral;
mod snapshot_lease;
mod snapshot_output;
mod snapshot_runtime;
mod snapshot_state;

fn cpath(path: &Path) -> Result<CString> {
    Ok(CString::new(path.as_os_str().as_bytes())?)
}

fn mount(
    source: Option<&Path>,
    target: &Path,
    kind: Option<&str>,
    flags: libc::c_ulong,
) -> Result<()> {
    let source = source.map(cpath).transpose()?;
    let target = cpath(target)?;
    let kind = kind.map(CString::new).transpose()?;
    // Called only after exec, in this worker's private mount namespace.
    if unsafe {
        libc::mount(
            source.as_ref().map_or(std::ptr::null(), |v| v.as_ptr()),
            target.as_ptr(),
            kind.as_ref().map_or(std::ptr::null(), |v| v.as_ptr()),
            flags,
            std::ptr::null(),
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error()).context("mount snapshot staging");
    }
    Ok(())
}

fn same_user(stream: &UnixStream) -> Result<()> {
    let mut credentials: libc::ucred = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of_val(&credentials) as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut length,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error()).context("read snapshot peer credentials");
    }
    if length as usize != std::mem::size_of_val(&credentials)
        || credentials.uid != unsafe { libc::geteuid() }
    {
        bail!("snapshot peer is not the owning user");
    }
    Ok(())
}

struct ExecutionControl<'a> {
    binding: &'a snapshot_lease::Binding,
    client: &'a UnixStream,
    timeout_ms: u64,
    max_output_bytes: usize,
}

fn execute(
    stage: &fs::File,
    root: &Path,
    argv: &[String],
    environment: &BTreeMap<String, String>,
    control: ExecutionControl<'_>,
    poisoned: &mut bool,
    runtime: Option<&snapshot_runtime::CapturedRuntime>,
) -> Result<serde_json::Value> {
    let ExecutionControl {
        binding,
        client,
        timeout_ms,
        max_output_bytes,
    } = control;
    if *poisoned {
        bail!("snapshot execution cleanup is unresolved");
    }
    if argv.is_empty()
        || !Path::new(&argv[0]).is_absolute()
        || !(1..=MAX_TIMEOUT_MS).contains(&timeout_ms)
        || !(1..=MAX_OUTPUT_BYTES).contains(&max_output_bytes)
    {
        bail!("absolute executable and supported execution/output limits required");
    }
    if runtime.is_none() && environment.contains_key("HERMES_HOME") {
        bail!("worker-state grants are not qualified by this root-only snapshot");
    }
    let deadline = Instant::now()
        + Duration::from_millis(timeout_ms)
            .min(binding.remaining(&binding.lease, snapshot_lease::now_ms()?)?);
    let mut command = Command::new(std::env::current_exe()?);
    command.arg("--launch-bwrap");
    if runtime.is_none() {
        command.args(["--ro-bind", "/usr", "/usr"]);
    }
    command
        .env_clear()
        .args([
            "--unshare-user",
            "--unshare-pid",
            "--unshare-ipc",
            "--unshare-uts",
            "--die-with-parent",
            "--disable-userns",
            "--cap-drop",
            "ALL",
            "--symlink",
            "usr/bin",
            "/bin",
            "--symlink",
            "usr/sbin",
            "/sbin",
            "--symlink",
            "usr/lib",
            "/lib",
            "--symlink",
            "usr/lib64",
            "/lib64",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/var/tmp",
            "--setenv",
            "TMPDIR",
            "/var/tmp",
            "--bind-fd",
        ])
        .arg(stage.as_raw_fd().to_string())
        .arg(root);
    if let Some(runtime) = runtime {
        runtime.mount_arguments(&mut command)?;
    }
    // Provider-controlled variables must never reach the dynamic loader of
    // bubblewrap itself. They are installed only inside the constructed sandbox.
    for (key, value) in environment {
        command.arg("--setenv").arg(key).arg(value);
    }
    command.arg("--chdir").arg(root).arg("--").args(argv);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    // No host descriptor capability or control socket may reach a provider.
    let parent_fd = unsafe { libc::syscall(libc::SYS_pidfd_open, libc::getpid(), 0) };
    if parent_fd == -1 {
        return Err(std::io::Error::last_os_error()).context("pin snapshot keeper lifetime");
    }
    let parent_fd = unsafe { OwnedFd::from_raw_fd(parent_fd as i32) };
    let parent_raw_fd = parent_fd.as_raw_fd();
    let mut inherited = runtime
        .map(|runtime| runtime.inherited_descriptors())
        .unwrap_or_default();
    inherited.push(stage.as_raw_fd());

    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            // getppid() is zero for PID 1 with a parent outside its namespace.
            // The inherited pidfd checks the exact parent without numeric reuse.
            let mut parent = libc::pollfd {
                fd: parent_raw_fd,
                events: libc::POLLIN,
                revents: 0,
            };
            if libc::poll(&mut parent, 1, 0) != 0 {
                return Err(std::io::Error::from_raw_os_error(libc::ECHILD));
            }
            if libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 4u32) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            for fd in &inherited {
                if libc::fcntl(*fd, libc::F_SETFD, 0) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    let mut child =
        snapshot_child::ProviderChild::new(command.spawn().context("spawn snapshot provider")?);
    // Any failure after spawn blocks subsequent execution and release until an
    // independent owner proves teardown; an error response is not cleanup proof.
    *poisoned = true;
    let (stdout, stderr) = child.take_output();
    let mut stdout =
        snapshot_output::Output::new(stdout.context("snapshot stdout missing")?, max_output_bytes)?;
    let mut stderr =
        snapshot_output::Output::new(stderr.context("snapshot stderr missing")?, max_output_bytes)?;
    let mut cancelled = false;
    let pgid = i32::try_from(child.id())?;
    let timed_out = loop {
        stdout.poll()?;
        stderr.poll()?;
        // Execute uses one request per connection. EOF (including write-half
        // close) cancels the command, not the retained capability.
        let mut byte = 0u8;
        let read = unsafe {
            libc::recv(
                client.as_raw_fd(),
                (&mut byte as *mut u8).cast(),
                1,
                libc::MSG_PEEK | libc::MSG_DONTWAIT,
            )
        };
        if read == 0 {
            cancelled = true;
        } else if read < 0 {
            let error = std::io::Error::last_os_error();
            if !matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
            ) {
                cancelled = true;
            }
        } else {
            // No pipelined commands are accepted on an executing connection.
            cancelled = true;
        }
        if cancelled || stdout.overflow || stderr.overflow {
            break false;
        }
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        // Leave the leader waitable until group termination: reaping it first
        // would permit numeric PID/PGID reuse before kill.
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                child.id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result == -1 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error).context("observe snapshot leader");
        }
        if unsafe { info.si_pid() } != 0 {
            break false;
        }
        if Instant::now() >= deadline
            || binding
                .remaining(&binding.lease, snapshot_lease::now_ms()?)
                .is_err()
        {
            break true;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    // Terminate remaining group members even if the leader exited first.
    let cleanup = child.begin_cleanup();
    if unsafe { libc::kill(-pgid, libc::SIGKILL) } == -1 {
        return Err(std::io::Error::last_os_error()).context("terminate snapshot group");
    }

    let status = loop {
        if let Some(status) = child.try_wait().context("reap snapshot provider")? {
            break status;
        }
        if Instant::now() >= cleanup {
            bail!("snapshot leader cleanup deadline exceeded");
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    loop {
        if unsafe { libc::kill(-pgid, 0) } == -1 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH) {
                break;
            }
            return Err(error).context("observe snapshot process group");
        }
        if Instant::now() >= cleanup {
            bail!("snapshot process group cleanup deadline exceeded");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    child.finish();
    // Tree teardown is proven above. Drain only the bounded residual pipe data.
    while stdout.poll()? {}
    while stderr.poll()? {}
    *poisoned = false;
    Ok(
        serde_json::json!({"ok": true, "timed_out": timed_out, "cancelled": cancelled,
        "output_limit": stdout.overflow || stderr.overflow,
        "stdout": String::from_utf8_lossy(&stdout.bytes),
        "stderr": String::from_utf8_lossy(&stderr.bytes), "code": status.code()}),
    )
}

mod snapshot_admission;

fn serve(
    state: &Path,
    supplied_root: &Path,
    admission: Option<snapshot_admission::Admission>,
    runtime: Option<snapshot_runtime::RuntimeAdmission>,
    expected_capture: Option<arda_engine::objectives::capture_envelope::ExpectedCaptureEnvelope>,
) -> Result<()> {
    // unshare normally establishes this already; repeat fail-closed before any
    // capture so incoming host mount propagation cannot modify the staged tree.
    mount(None, Path::new("/"), None, libc::MS_REC | libc::MS_PRIVATE)?;
    let root = supplied_root.canonicalize()?;
    if let Some(admission) = &admission {
        admission.after_clone(&root)?;
    }
    if !root.is_dir() || root == Path::new("/") {
        bail!("snapshot root must be a non-root directory");
    }
    let exposed = if let Some(runtime) = &runtime {
        runtime.after_clone()?;
        let mut devices = runtime.exposed_devices()?;
        devices.extend(
            admission
                .as_ref()
                .context("configured runtime requires workspace admission")?
                .exposed_devices()?,
        );
        Some(devices)
    } else {
        None
    };
    let mut owned_state =
        snapshot_state::StateDirectory::create_excluding(state, &root, exposed.as_ref())?;
    let state = owned_state.path();
    let listener = UnixListener::bind(state.join("control.sock"))?;
    let staging = state.join("staging");
    fs::create_dir(&staging)?;
    mount(
        None,
        &staging,
        Some("tmpfs"),
        libc::MS_NODEV | libc::MS_NOSUID,
    )?;
    let stage = staging.join("root");
    owned_state.mark_staging_mounted();
    fs::create_dir(&stage)?;
    mount(Some(&root), &stage, None, libc::MS_BIND | libc::MS_REC)?;
    if let Some(admission) = &admission {
        admission.check_capture(&stage)?;
    }
    let stage = fs::File::open(&stage)?;
    // Staging is inaccessible from the host namespace and survives host renames.
    let metadata = stage.metadata()?;

    let runtime = runtime
        .map(|admission| admission.capture(&staging, fs::metadata(&state)?.dev()))
        .transpose()?;
    let mut manifest = Manifest {
        runtime_bundle: runtime.as_ref().map(|runtime| runtime.manifest()),
        admission_digest: None,
        version: if runtime.is_some() { 2 } else { 1 },
        capability: uuid::Uuid::new_v4().to_string(),
        root: root.clone(),
        device: metadata.dev(),
        inode: metadata.ino(),
        topology_digest: format!("{:x}", Sha256::digest(fs::read("/proc/self/mountinfo")?)),
    };
    if let Some(expected) = &expected_capture {
        let runtime = runtime
            .as_ref()
            .context("capture envelope requires runtime policy")?;
        let workspace_witness = snapshot_admission::physical::Physical::capture_witness(&stage)?;
        let grants = runtime
            .grants
            .iter()
            .map(|grant| snapshot_admission::physical::Physical::capture_witness(&grant.descriptor))
            .collect::<Result<Vec<_>>>()?;
        expected.verify_captures(&runtime.policy, &workspace_witness, &grants)?;
        manifest.topology_digest = workspace_witness.digest()?;
        manifest.admission_digest = Some(expected.digest()?);
    }
    manifest.validate_runtime(runtime.as_ref().map(|runtime| &runtime.policy))?;
    let encoded = serde_json::to_vec(&manifest)?;
    let digest = format!("{:x}", Sha256::digest(&encoded));
    let mut committed: Option<snapshot_lease::Binding> = None;
    let mut poisoned = false;
    for stream in listener.incoming() {
        let mut stream = stream?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        if same_user(&stream).is_err() {
            continue;
        }
        let mut release = false;
        let response = (|| -> Result<serde_json::Value> {
            let line = read_request(&mut stream)?;
            match serde_json::from_slice::<Request>(&line)? {
                Request::Inspect => Ok(serde_json::json!({"ok": true, "manifest": manifest, "manifest_digest": digest, "committed": committed.as_ref().map(|b| &b.lease)})),
                Request::Commit { capability, manifest_digest, lease } => {
                    if capability != manifest.capability || manifest_digest != digest { bail!("snapshot admission binding mismatch"); }
                    if poisoned { bail!("snapshot execution cleanup is unresolved; rebind refused"); }
                    // Serialized requests acknowledge rebind only after earlier
                    // execution has completed its bounded teardown.
                    snapshot_lease::Binding::commit(&mut committed, lease, snapshot_lease::now_ms()?)?;
                    Ok(serde_json::json!({"ok": true}))
                }
                Request::Execute { capability, lease, argv, environment, timeout_ms, max_output_bytes } => {
                    if runtime.is_some() { bail!("configured runtime refuses untyped executable dispatch"); }
                    if capability != manifest.capability { bail!("snapshot execution is not admitted"); }
                    let binding = committed.as_ref().context("snapshot execution is not admitted")?;
                    binding.remaining(&lease, snapshot_lease::now_ms()?)?;
                    execute(&stage, &root, &argv, &environment, ExecutionControl { binding, client: &stream, timeout_ms, max_output_bytes }, &mut poisoned, None)
                }
                Request::Runtime { capability, lease, operation, timeout_ms, max_output_bytes } => {
                    if manifest.admission_digest.is_none() || capability != manifest.capability { bail!("typed runtime needs witnessed admission"); }
                    let runtime = runtime.as_ref().context("typed runtime requires configured grants")?;
                    let binding = committed.as_ref().context("snapshot execution is not admitted")?;
                    binding.remaining(&lease, snapshot_lease::now_ms()?)?;
                    let argv = runtime.operation_argv(&operation)?;
                    execute(&stage, &root, &argv, &runtime.policy.policy().fixed_environment,
                        ExecutionControl { binding, client: &stream, timeout_ms, max_output_bytes }, &mut poisoned, Some(runtime))
                }
                Request::Release { capability } => {
                    if capability != manifest.capability { bail!("snapshot capability mismatch"); }
                    if poisoned { bail!("snapshot execution cleanup is unresolved; release refused"); }
                    release = true;
                    Ok(serde_json::json!({"ok": true}))
                }
            }
        })().unwrap_or_else(|error| serde_json::json!({"ok": false, "error": format!("{error:#}")}));
        if release {
            drop(listener);
            drop(stage);
            drop(runtime);
            let cleanup = owned_state.cleanup();
            let response = match &cleanup {
                Ok(()) => serde_json::json!({"ok": true}),
                Err(error) => serde_json::json!({"ok": false, "error": format!("{error:#}")}),
            };
            let _ = writeln!(stream, "{}", response);
            return cleanup;
        }
        // A client disconnect never destroys the snapshot or undoes admission.
        let _ = writeln!(stream, "{}", response);
    }
    drop(listener);
    drop(stage);
    owned_state.cleanup()
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--capture-ephemeral") {
        return snapshot_ephemeral::capture(&args[1..]);
    }
    if args.first().is_some_and(|arg| arg == "--launch-captured") {
        return snapshot_ephemeral::launch(&args[1..]);
    }
    if args.first().is_some_and(|arg| arg == "--launch-bwrap") {
        return launch_bwrap(&args[1..]);
    }
    if !(2..=5).contains(&args.len()) {
        bail!("usage: arda-snapshot-worker NEW_STATE_DIRECTORY WORKSPACE [ADMISSION_IDENTITY]");
    }
    let admission = args
        .get(2)
        .map(|identity| {
            snapshot_admission::Admission::before_clone(
                Path::new(&args[1]),
                identity
                    .to_str()
                    .context("admission identity must be UTF-8")?,
            )
        })
        .transpose()?;
    let runtime = args
        .get(3)
        .map(|fd| -> Result<_> {
            let fd = fd.to_str().context("policy fd")?.parse::<i32>()?;
            if fd <= 2 {
                bail!("invalid policy descriptor");
            }
            let policy = arda_engine::objectives::runtime_policy::transport::read(unsafe {
                OwnedFd::from_raw_fd(fd)
            })?;
            snapshot_runtime::RuntimeAdmission::before_clone(policy, Path::new(&args[1]))
        })
        .transpose()?;
    // This dedicated binary is single-threaded. No alternate/internal entry
    // point can perform mount mutations in the caller's namespace.
    let expected_capture = args
        .get(4)
        .map(|fd| -> Result<_> {
            let fd = fd.to_str().context("envelope fd")?.parse::<i32>()?;
            let policy_fd = args[3].to_str().context("policy fd")?.parse::<i32>()?;
            if fd <= 2 || fd == policy_fd {
                bail!("invalid envelope descriptor");
            }
            let policy = runtime
                .as_ref()
                .context("envelope requires policy")?
                .policy();
            arda_engine::objectives::capture_envelope::ExpectedCaptureEnvelope::read_worker(
                unsafe { OwnedFd::from_raw_fd(fd) },
                policy,
                args[2].to_str().context("admission identity")?,
            )
        })
        .transpose()?;
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };
    if unsafe { libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNS) } == -1 {
        return Err(std::io::Error::last_os_error()).context("isolate snapshot namespace");
    }
    fs::write("/proc/self/setgroups", "deny")?;
    fs::write("/proc/self/uid_map", format!("0 {uid} 1\n"))?;
    fs::write("/proc/self/gid_map", format!("0 {gid} 1\n"))?;
    serve(
        Path::new(&args[0]),
        Path::new(&args[1]),
        admission,
        runtime,
        expected_capture,
    )
}

fn launch_bwrap(args: &[std::ffi::OsString]) -> Result<()> {
    // unshare(CLONE_NEWPID) cannot be repeated by the same process. An execed,
    // single-use launcher keeps namespace setup out of pre_exec and gives each
    // execution a fresh PID-1 lifetime boundary around bubblewrap's bootstrap.
    if unsafe { libc::unshare(libc::CLONE_NEWPID) } == -1 {
        return Err(std::io::Error::last_os_error()).context("isolate provider process lifetime");
    }
    let parent_fd = unsafe { libc::syscall(libc::SYS_pidfd_open, libc::getpid(), 0) };
    if parent_fd == -1 {
        return Err(std::io::Error::last_os_error()).context("pin launcher lifetime");
    }
    let parent_fd = unsafe { OwnedFd::from_raw_fd(parent_fd as i32) };
    let parent_raw_fd = parent_fd.as_raw_fd();
    let mut command = Command::new("/usr/bin/bwrap");
    command.env_clear().args(args);
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            let mut parent = libc::pollfd {
                fd: parent_raw_fd,
                events: libc::POLLIN,
                revents: 0,
            };
            if libc::poll(&mut parent, 1, 0) != 0 {
                return Err(std::io::Error::from_raw_os_error(libc::ECHILD));
            }
            Ok(())
        });
    }
    let status = command.status().context("run namespace-init bubblewrap")?;
    std::process::exit(status.code().unwrap_or(1));
}

fn read_request(stream: &mut UnixStream) -> Result<Vec<u8>> {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut line = Vec::new();
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .context("snapshot request deadline exceeded")?;
        if remaining.is_zero() {
            bail!("snapshot request deadline exceeded");
        }
        stream.set_read_timeout(Some(remaining))?;
        let mut byte = [0];
        if stream.read(&mut byte)? == 0 {
            bail!("incomplete snapshot request");
        }
        line.push(byte[0]);
        if line.len() > MAX_REQUEST_BYTES {
            bail!("snapshot request exceeds limit");
        }
        if byte[0] == b'\n' {
            return Ok(line);
        }
    }
}
