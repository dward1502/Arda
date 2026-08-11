use std::time::Duration;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::browser_capture::{
    loopback_mjpeg_url, BrowserCaptureState, BrowserLaunchPlan, BrowserRuntime, FrameHub,
    StartBrowserCaptureRequest, StopBrowserCaptureRequest,
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
    assert!(plan
        .args
        .iter()
        .any(|arg| arg == "--remote-debugging-address=127.0.0.1"));
    assert!(plan
        .args
        .iter()
        .any(|arg| arg == "--user-data-dir=/tmp/arda-hud-browser-browser-session-a"));
    assert_eq!(plan.args.last().map(String::as_str), Some("https://example.com/"));
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
    assert!(hub.wait_for_revision_after(0, Duration::from_millis(1)).is_none());

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
#[ignore = "requires an installed Chromium runtime"]
fn live_browser_capture_publishes_changing_frames_and_stops_its_process() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("test page listener");
    listener.set_nonblocking(true).expect("nonblocking listener");
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

    std::thread::sleep(Duration::from_secs(2));
    let later = state
        .status("browser-live-gate")
        .expect("owned browser should remain live");
    assert!(later.frame_revision > started.frame_revision);

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
