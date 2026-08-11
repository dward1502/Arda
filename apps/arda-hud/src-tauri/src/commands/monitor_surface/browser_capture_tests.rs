use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::browser_capture::{
    authorize_browser_control, browser_click_commands, browser_key_commands,
    browser_scroll_commands, browser_text_commands, loopback_mjpeg_url, BrowserCaptureDescriptor,
    BrowserCaptureState, BrowserLaunchPlan, BrowserRuntime, ClickBrowserCaptureRequest, FrameHub,
    NavigateBrowserCaptureRequest, StartBrowserCaptureRequest, StopBrowserCaptureRequest,
};

#[test]
fn browser_launch_plan_isolates_profile_and_forces_audio_mute() {
    let plan = BrowserLaunchPlan::new(
        BrowserRuntime::FlatpakBrave,
        "browser-session-a",
        "https://example.com/",
        9333,
    )
    .expect("valid browser plan");

    assert!(plan.args.iter().any(|arg| arg == "--mute-audio"));
    assert!(plan.args.iter().any(|arg| arg == "--headless=new"));
    assert!(plan.args.iter().any(|arg| {
        arg.starts_with("--user-agent=Mozilla/5.0 (X11; Linux x86_64)")
            && arg.contains(" Chrome/")
            && !arg.contains("HeadlessChrome")
    }));
    assert!(plan
        .args
        .iter()
        .any(|arg| arg == "--remote-debugging-address=127.0.0.1"));
    assert!(plan
        .args
        .iter()
        .any(|arg| arg == "--user-data-dir=/tmp/arda-hud-browser-browser-session-a"));
    assert_eq!(
        plan.args.last().map(String::as_str),
        Some("https://example.com/")
    );
}

#[test]
fn browser_launch_plan_rejects_non_http_navigation() {
    let error = BrowserLaunchPlan::new(
        BrowserRuntime::FlatpakBrave,
        "browser-session-a",
        "file:///tmp/staged-browser.png",
        9333,
    )
    .expect_err("file URLs cannot become browser capability evidence");

    assert!(error.contains("HTTP(S)"));
}

#[test]
fn stream_url_is_loopback_and_session_scoped() {
    assert_eq!(
        loopback_mjpeg_url(41827, "browser-session-a"),
        "http://127.0.0.1:41827/session/browser-session-a.mjpeg"
    );
}

#[test]
fn frame_hub_reports_only_real_published_frame_revisions() {
    let hub = FrameHub::default();
    assert!(hub
        .wait_for_revision_after(0, Duration::from_millis(1))
        .is_none());

    assert_eq!(hub.publish(vec![0xff, 0xd8, 1, 0xff, 0xd9]), 1);
    let first = hub
        .wait_for_revision_after(0, Duration::from_millis(10))
        .expect("first live frame");
    assert_eq!(first.revision, 1);

    assert_eq!(hub.publish(vec![0xff, 0xd8, 2, 0xff, 0xd9]), 2);
    let second = hub
        .wait_for_revision_after(first.revision, Duration::from_millis(10))
        .expect("changed live frame");
    assert_eq!(second.revision, 2);
    assert_ne!(first.jpeg, second.jpeg);
}

#[test]
fn frame_hub_returns_the_latest_frame_only_after_the_requested_revision() {
    let hub = FrameHub::default();
    hub.publish(vec![0xff, 0xd8, 1, 0xff, 0xd9]);
    hub.publish(vec![0xff, 0xd8, 2, 0xff, 0xd9]);

    let latest = hub.latest_after(0).expect("latest native frame");
    assert_eq!(latest.revision, 2);
    assert_eq!(latest.jpeg, vec![0xff, 0xd8, 2, 0xff, 0xd9]);
    assert!(hub.latest_after(2).is_none());
}

fn capture_descriptor(owner: &str, revision: u64) -> BrowserCaptureDescriptor {
    BrowserCaptureDescriptor {
        session_id: "browser-session-a".to_string(),
        owner: owner.to_string(),
        revision,
        url: "https://example.com/".to_string(),
        stream_url: loopback_mjpeg_url(41827, "browser-session-a"),
        transport: "mjpeg",
        muted: true,
        process_id: 42,
        frame_revision: 2,
    }
}

#[test]
fn browser_control_requires_exact_owner_and_revision() {
    let descriptor = capture_descriptor("agent:browser-a", 7);

    authorize_browser_control(&descriptor, "agent:browser-a", 7)
        .expect("exact owner and revision should control the browser");
    assert!(authorize_browser_control(&descriptor, "agent:browser-b", 7)
        .expect_err("another owner must be rejected")
        .contains("owner mismatch"));
    assert!(authorize_browser_control(&descriptor, "agent:browser-a", 6)
        .expect_err("stale browser revision must be rejected")
        .contains("revision conflict"));
}

#[test]
fn browser_pointer_input_is_bounded_and_emits_a_complete_click() {
    let commands = browser_click_commands(640.0, 360.0)
        .expect("a point inside the capture viewport should be accepted");
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0]["method"], "Input.dispatchMouseEvent");
    assert_eq!(commands[0]["params"]["type"], "mousePressed");
    assert_eq!(commands[1]["params"]["type"], "mouseReleased");
    assert!(browser_click_commands(-1.0, 10.0).is_err());
    assert!(browser_click_commands(1281.0, 10.0).is_err());
    assert!(browser_click_commands(10.0, f64::NAN).is_err());
}

#[test]
fn browser_workstation_input_emits_wheel_text_and_special_keys() {
    let wheel = browser_scroll_commands(640.0, 360.0, 0.0, 240.0)
        .expect("finite wheel input inside the capture viewport should be accepted");
    assert_eq!(wheel[0]["method"], "Input.dispatchMouseEvent");
    assert_eq!(wheel[0]["params"]["type"], "mouseWheel");
    assert_eq!(wheel[0]["params"]["deltaY"], 240.0);
    assert!(browser_scroll_commands(640.0, 360.0, f64::NAN, 1.0).is_err());

    let text = browser_text_commands("Arda monitor")
        .expect("ordinary workstation text should be accepted");
    assert_eq!(text[0]["method"], "Input.insertText");
    assert_eq!(text[0]["params"]["text"], "Arda monitor");
    assert!(browser_text_commands("").is_err());

    let key = browser_key_commands("Enter").expect("supported editing keys should be accepted");
    assert_eq!(key.len(), 2);
    assert_eq!(key[0]["params"]["type"], "keyDown");
    assert_eq!(key[1]["params"]["type"], "keyUp");
    assert!(browser_key_commands("F12").is_err());
}

#[test]
#[ignore = "requires an installed Chromium runtime"]
fn live_browser_capture_publishes_changing_frames_and_stops_its_process() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("test page listener");
    listener
        .set_nonblocking(true)
        .expect("nonblocking listener");
    let page_port = listener.local_addr().expect("test page address").port();
    let page_shutdown = Arc::new(AtomicBool::new(false));
    let page_shutdown_thread = page_shutdown.clone();
    let page_thread = std::thread::spawn(move || {
        let body = r#"<!doctype html><html><body><script>
          let frame = 0;
          setInterval(() => {
            frame += 1;
            document.body.style.backgroundColor = `hsl(${frame % 360} 80% 35%)`;
            document.body.textContent = `ARDA LIVE FRAME ${frame}`;
          }, 40);
        </script></body></html>"#;
        while !page_shutdown_thread.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut request = [0_u8; 1024];
                    let _ = stream.read(&mut request);
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(), body
                    );
                    let _ = stream.write_all(response.as_bytes());
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("test page server failed: {error}"),
            }
        }
    });
    let state = BrowserCaptureState::default();
    let started = state
        .start(StartBrowserCaptureRequest {
            session_id: "browser-live-gate".to_string(),
            owner: "browser-live-owner".to_string(),
            url: format!("http://127.0.0.1:{page_port}/live"),
        })
        .expect("real browser capture should start only after two frames");
    assert!(started.muted);
    assert_eq!(started.transport, "mjpeg");
    assert!(started.frame_revision >= 2);

    let navigated_url = format!("http://127.0.0.1:{page_port}/navigated");
    let navigated = state
        .navigate(NavigateBrowserCaptureRequest {
            session_id: started.session_id.clone(),
            owner: started.owner.clone(),
            expected_revision: started.revision,
            url: navigated_url.clone(),
        })
        .expect("the exact owner and revision should navigate the live browser");
    assert_eq!(navigated.url, navigated_url);
    assert_eq!(navigated.revision, started.revision + 1);
    assert!(state
        .navigate(NavigateBrowserCaptureRequest {
            session_id: started.session_id.clone(),
            owner: started.owner.clone(),
            expected_revision: started.revision,
            url: format!("http://127.0.0.1:{page_port}/stale"),
        })
        .expect_err("a stale revision must not navigate the live browser")
        .contains("revision conflict"));

    let clicked = state
        .click(ClickBrowserCaptureRequest {
            session_id: navigated.session_id.clone(),
            owner: navigated.owner.clone(),
            expected_revision: navigated.revision,
            x: 640.0,
            y: 360.0,
        })
        .expect("the exact owner and revision should dispatch live pointer input");
    assert_eq!(clicked.revision, navigated.revision + 1);

    std::thread::sleep(Duration::from_secs(2));
    let later = state
        .status("browser-live-gate")
        .expect("owned browser should remain live");
    assert!(later.frame_revision > clicked.frame_revision);

    state
        .stop(StopBrowserCaptureRequest {
            session_id: "browser-live-gate".to_string(),
            owner: "browser-live-owner".to_string(),
        })
        .expect("owner should stop its browser capture");
    assert!(state.status("browser-live-gate").is_err());
    page_shutdown.store(true, Ordering::Release);
    page_thread.join().expect("test page server should stop");
}

#[test]
#[ignore = "requires an installed Chromium runtime"]
fn two_live_browser_captures_are_concurrent_and_lifecycle_isolated() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("test page listener");
    listener
        .set_nonblocking(true)
        .expect("nonblocking listener");
    let page_port = listener.local_addr().expect("test page address").port();
    let page_shutdown = Arc::new(AtomicBool::new(false));
    let page_shutdown_thread = page_shutdown.clone();
    let page_thread = std::thread::spawn(move || {
        let body = r#"<!doctype html><html><body><script>
          let frame = 0;
          setInterval(() => {
            frame += 1;
            document.body.style.backgroundColor = `hsl(${frame % 360} 80% 35%)`;
            document.body.textContent = `ARDA CONCURRENT FRAME ${frame}`;
          }, 40);
        </script></body></html>"#;
        while !page_shutdown_thread.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut request = [0_u8; 1024];
                    let _ = stream.read(&mut request);
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(), body
                    );
                    let _ = stream.write_all(response.as_bytes());
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("test page server failed: {error}"),
            }
        }
    });

    let process_id = std::process::id();
    let first_session_id = format!("browser-concurrent-a-{process_id}");
    let second_session_id = format!("browser-concurrent-b-{process_id}");
    let first_profile = format!("/tmp/arda-hud-browser-{first_session_id}");
    let second_profile = format!("/tmp/arda-hud-browser-{second_session_id}");
    let state = BrowserCaptureState::default();
    let barrier = Arc::new(std::sync::Barrier::new(3));

    let first_state = state.clone();
    let first_barrier = barrier.clone();
    let first_url = format!("http://127.0.0.1:{page_port}/first");
    let first_id = first_session_id.clone();
    let first_start = std::thread::spawn(move || {
        first_barrier.wait();
        first_state.start(StartBrowserCaptureRequest {
            session_id: first_id,
            owner: "browser-concurrent-owner-a".to_string(),
            url: first_url,
        })
    });

    let second_state = state.clone();
    let second_barrier = barrier.clone();
    let second_url = format!("http://127.0.0.1:{page_port}/second");
    let second_id = second_session_id.clone();
    let second_start = std::thread::spawn(move || {
        second_barrier.wait();
        second_state.start(StartBrowserCaptureRequest {
            session_id: second_id,
            owner: "browser-concurrent-owner-b".to_string(),
            url: second_url,
        })
    });

    barrier.wait();
    let first = first_start
        .join()
        .expect("first startup thread should finish")
        .expect("first browser should publish changing frames");
    let second = second_start
        .join()
        .expect("second startup thread should finish")
        .expect("second browser should publish changing frames");

    assert_ne!(first.process_id, second.process_id);
    assert_ne!(first.stream_url, second.stream_url);
    assert_ne!(first.owner, second.owner);
    assert!(first.muted && second.muted);
    assert!(first.frame_revision >= 2 && second.frame_revision >= 2);
    assert!(std::path::Path::new(&first_profile).is_dir());
    assert!(std::path::Path::new(&second_profile).is_dir());

    assert!(state
        .stop(StopBrowserCaptureRequest {
            session_id: first_session_id.clone(),
            owner: second.owner.clone(),
        })
        .expect_err("one browser owner must not stop the other browser")
        .contains("owner mismatch"));

    state
        .stop(StopBrowserCaptureRequest {
            session_id: first_session_id.clone(),
            owner: first.owner.clone(),
        })
        .expect("first owner should stop only the first browser");
    assert!(state.status(&first_session_id).is_err());
    assert!(!std::path::Path::new(&first_profile).exists());

    std::thread::sleep(Duration::from_millis(200));
    let second_later = state
        .status(&second_session_id)
        .expect("second browser must remain live after first-browser cleanup");
    assert!(second_later.frame_revision > second.frame_revision);
    assert!(std::path::Path::new(&second_profile).is_dir());

    state
        .stop(StopBrowserCaptureRequest {
            session_id: second_session_id.clone(),
            owner: second.owner,
        })
        .expect("second owner should stop the second browser");
    assert!(state.status(&second_session_id).is_err());
    assert!(!std::path::Path::new(&second_profile).exists());

    std::thread::sleep(Duration::from_millis(300));
    assert!(
        !std::path::Path::new(&first_profile).exists(),
        "first browser descendants must not recreate the deleted profile"
    );
    assert!(
        !std::path::Path::new(&second_profile).exists(),
        "second browser descendants must not recreate the deleted profile"
    );

    page_shutdown.store(true, Ordering::Release);
    page_thread.join().expect("test page server should stop");
}
