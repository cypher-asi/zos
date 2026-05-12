use std::net::TcpListener as StdTcpListener;
use std::path::PathBuf;

use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::window::{Icon, WindowBuilder};
use tokio::net::TcpListener;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use wry::{WebContext, WebViewBuilder};

const PREFERRED_PORT: u16 = 19847;

// ---------------------------------------------------------------------------
// IPC
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum UserEvent {
    WindowCommand { cmd: WinCmd },
    ShowWindow,
}

#[derive(Debug)]
enum WinCmd {
    Minimize,
    Maximize,
    Close,
    Drag,
}

fn ipc_handler(
    proxy: EventLoopProxy<UserEvent>,
) -> impl Fn(wry::http::Request<String>) + 'static {
    move |req: wry::http::Request<String>| {
        let msg = req.body().trim();
        match msg {
            "ready" => {
                debug!("IPC ready signal");
                let _ = proxy.send_event(UserEvent::ShowWindow);
            }
            "minimize" | "maximize" | "close" | "drag" => {
                let cmd = match msg {
                    "minimize" => WinCmd::Minimize,
                    "maximize" => WinCmd::Maximize,
                    "close" => WinCmd::Close,
                    "drag" => WinCmd::Drag,
                    _ => unreachable!(),
                };
                debug!(command = msg, "IPC event");
                let _ = proxy.send_event(UserEvent::WindowCommand { cmd });
            }
            other => warn!(message = other, "unknown IPC message"),
        }
    }
}

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

fn set_square_corners(_window: &tao::window::Window) {
    #[cfg(target_os = "windows")]
    {
        use tao::platform::windows::WindowExtWindows;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWM_WINDOW_CORNER_PREFERENCE,
        };
        let hwnd = HWND(_window.hwnd() as *mut std::ffi::c_void);
        let pref = DWM_WINDOW_CORNER_PREFERENCE(1); // DWMWCP_DONOTROUND
        let _ = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &pref as *const _ as *const _,
                std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
            )
        };
    }

    #[cfg(target_os = "macos")]
    {
        use objc::{sel, sel_impl};
        use tao::platform::macos::WindowExtMacOS;
        unsafe {
            let ns_window = _window.ns_window() as *mut objc::runtime::Object;
            let content_view: *mut objc::runtime::Object =
                objc::msg_send![ns_window, contentView];
            let _: () = objc::msg_send![content_view, setWantsLayer: true];
            let layer: *mut objc::runtime::Object = objc::msg_send![content_view, layer];
            let _: () = objc::msg_send![layer, setCornerRadius: 0.0_f64];
            let _: () = objc::msg_send![layer, setMasksToBounds: true];
        }
    }
}

fn load_icon() -> Icon {
    let png_bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(png_bytes).expect("failed to decode icon");
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), w, h).expect("failed to create icon")
}

fn create_window(
    event_loop: &tao::event_loop::EventLoop<UserEvent>,
    icon: Icon,
) -> tao::window::Window {
    let window = WindowBuilder::new()
        .with_title("ZERO")
        .with_decorations(false)
        .with_visible(false)
        .with_window_icon(Some(icon))
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 800.0))
        .build(event_loop)
        .expect("failed to build window");
    set_square_corners(&window);
    info!("window created");
    window
}

const READY_SCRIPT: &str = "\
    if (document.readyState === 'loading') { \
        document.addEventListener('DOMContentLoaded', function() { window.ipc.postMessage('ready'); }); \
    } else { \
        window.ipc.postMessage('ready'); \
    }";

fn create_webview(
    window: &tao::window::Window,
    web_context: &mut WebContext,
    url: &str,
    proxy: EventLoopProxy<UserEvent>,
) -> wry::WebView {
    let builder = WebViewBuilder::new_with_web_context(web_context)
        .with_background_color((0, 0, 0, 255))
        .with_url(url)
        .with_initialization_script(READY_SCRIPT)
        .with_ipc_handler(ipc_handler(proxy))
        .with_new_window_req_handler(|uri, _features| {
            let _ = open::that(&uri);
            wry::NewWindowResponse::Deny
        });

    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(window).expect("failed to build webview");

    #[cfg(target_os = "linux")]
    let webview = {
        use wry::WebViewBuilderExtUnix;
        builder
            .build_gtk(window.gtk_window())
            .expect("failed to build webview")
    };

    webview
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

fn find_interface_dir() -> Option<PathBuf> {
    let compile_time = PathBuf::from(env!("INTERFACE_DIST_DIR"));
    if compile_time.join("index.html").exists() {
        return Some(compile_time);
    }
    ["interface/dist", "../../interface/dist"]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.join("index.html").exists())
}

fn bind_listener() -> (StdTcpListener, String) {
    let listener = StdTcpListener::bind(format!("127.0.0.1:{PREFERRED_PORT}"))
        .or_else(|_| StdTcpListener::bind("127.0.0.1:0"))
        .expect("failed to bind");
    listener.set_nonblocking(true).expect("set_nonblocking");
    let port = listener.local_addr().expect("local_addr").port();
    let url = format!("http://127.0.0.1:{port}");
    info!(%url, "server binding ready");
    (listener, url)
}

fn spawn_server(
    std_listener: StdTcpListener,
    interface_dir: Option<PathBuf>,
) -> std::sync::mpsc::Receiver<()> {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            let app = zero_server::create_router(interface_dir);
            let listener = TcpListener::from_std(std_listener).expect("from_std");
            let _ = tx.send(());
            axum::serve(listener, app).await.expect("server error");
        });
    });
    rx
}

// ---------------------------------------------------------------------------
// Event loop
// ---------------------------------------------------------------------------

fn run_event_loop(
    event_loop: tao::event_loop::EventLoop<UserEvent>,
    window: tao::window::Window,
) {
    event_loop.run(move |event, _elwt, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::UserEvent(ue) => match ue {
                UserEvent::ShowWindow => window.set_visible(true),
                UserEvent::WindowCommand { cmd } => match cmd {
                    WinCmd::Minimize => window.set_minimized(true),
                    WinCmd::Maximize => window.set_maximized(!window.is_maximized()),
                    WinCmd::Close => *control_flow = ControlFlow::Exit,
                    WinCmd::Drag => {
                        let _ = window.drag_window();
                    }
                },
            },
            _ => {}
        }
    });
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("zero_desktop=debug,zero_server=debug,info")),
        )
        .init();

    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("zero");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let webview_data_dir = data_dir.join("webview");

    let interface_dir = find_interface_dir();
    match interface_dir {
        Some(ref d) => info!(path = %d.display(), "serving interface"),
        None => warn!("no interface dist found"),
    }

    let (std_listener, url) = bind_listener();
    let ready_rx = spawn_server(std_listener, interface_dir);
    ready_rx.recv().expect("server failed to start");
    info!("server ready");

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let icon = load_icon();
    let window = create_window(&event_loop, icon);
    let mut web_context = WebContext::new(Some(webview_data_dir));
    let _webview = create_webview(&window, &mut web_context, &url, proxy.clone());

    // Fallback: show window after 500ms even if IPC ready doesn't fire
    let show_proxy = proxy.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = show_proxy.send_event(UserEvent::ShowWindow);
    });

    run_event_loop(event_loop, window);
}
