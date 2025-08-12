use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use screenshots::Screen;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayInfo {
    pub index: usize,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

#[derive(Debug, Deserialize)]
pub struct CaptureArgs {
    pub rect: (i32, i32, i32, i32), // 逻辑像素：x,y,w,h（相对 overlay 左上）
    pub scale_factor: f64,          // 该屏缩放
    pub display_index: usize,       // 屏序号
}

/// 获取所有显示器信息
pub fn get_displays() -> Result<Vec<DisplayInfo>, String> {
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    
    let mut displays = Vec::new();
    for (index, screen) in screens.iter().enumerate() {
        displays.push(DisplayInfo {
            index,
            name: format!("Display {}", index + 1),
            x: screen.display_info.x,
            y: screen.display_info.y,
            width: screen.display_info.width,
            height: screen.display_info.height,
            scale_factor: screen.display_info.scale_factor as f64,
        });
    }
    
    Ok(displays)
}

/// 创建所有显示器的遮罩窗口
#[tauri::command]
pub async fn open_overlays_for_all_displays(app: AppHandle) -> Result<(), String> {
    let displays = get_displays()?;
    
    for display in displays {
        let label = format!("snip-overlay-{}", display.index);
        let url = format!("/overlay?i={}", display.index);
        
        // 检查窗口是否已存在，如果存在则关闭
        if let Some(existing_window) = app.get_window(&label) {
            let _ = existing_window.close();
        }
        
        // 创建新的遮罩窗口
        let _window = tauri::WindowBuilder::new(
            &app,
            &label,
            tauri::WindowUrl::App(url.parse().unwrap())
        )
        .title("")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .resizable(false)
        .inner_size(display.width as f64, display.height as f64)
        .position(display.x as f64, display.y as f64)
        .focused(true)
        .build()
        .map_err(|e| format!("Failed to create overlay window: {}", e))?;
    }
    
    Ok(())
}

/// 完成区域截图
#[tauri::command]
pub async fn complete_capture(args: CaptureArgs) -> Result<String, String> {
    #[cfg(debug_assertions)] println!("🔍 开始截图，参数: {:?}", args);

    // 获取所有屏幕
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    #[cfg(debug_assertions)] println!("📺 找到 {} 个屏幕", screens.len());

    let screen = screens.get(args.display_index)
        .ok_or_else(|| format!("Display index {} out of range", args.display_index))?;

    #[cfg(debug_assertions)] println!("🖥️ 使用屏幕 {}: {}x{}", args.display_index, screen.display_info.width, screen.display_info.height);

    // 计算实际截图区域（逻辑像素 -> 物理像素）
    let (x, y, w, h) = args.rect;
    #[cfg(debug_assertions)] println!("📐 逻辑像素区域: x={}, y={}, w={}, h={}", x, y, w, h);

    // 转换为物理像素坐标
    let physical_x = (x as f64 * args.scale_factor) as i32;
    let physical_y = (y as f64 * args.scale_factor) as i32;
    let physical_w = (w as f64 * args.scale_factor) as u32;
    let physical_h = (h as f64 * args.scale_factor) as u32;

    #[cfg(debug_assertions)] println!("🔍 物理像素区域: x={}, y={}, w={}, h={}", physical_x, physical_y, physical_w, physical_h);

    // 截取指定区域
    #[cfg(debug_assertions)] println!("📸 开始截取屏幕区域...");
    let img = screen.capture_area(physical_x, physical_y, physical_w, physical_h)
        .map_err(|e| format!("Failed to capture area: {}", e))?;
    
    // 保存图像
    #[cfg(debug_assertions)] println!("💾 图像尺寸: {}x{}", img.width(), img.height());
    let save_path = save_screenshot_image(&img)?;
    #[cfg(debug_assertions)] println!("✅ 截图保存到: {}", save_path);

    Ok(save_path)
}

/// 保存截图图像到本地
fn save_screenshot_image(img: &screenshots::Image) -> Result<String, String> {
    // 获取保存目录
    let save_dir = get_save_directory().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&save_dir).map_err(|e| format!("Failed to create directory: {}", e))?;
    
    // 生成文件名
    let filename = format!("region_capture_{}.png", Uuid::new_v4());
    let file_path = save_dir.join(filename);
    
    // 将图像转换为PNG格式并保存
    let png_data = img.to_png(None).map_err(|e| format!("Failed to convert to PNG: {}", e))?;
    std::fs::write(&file_path, png_data).map_err(|e| format!("Failed to write file: {}", e))?;
    
    Ok(file_path.to_string_lossy().to_string())
}

/// 获取保存目录
fn get_save_directory() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let pictures_dir = dirs::picture_dir()
        .or_else(|| dirs::home_dir())
        .ok_or("Could not find pictures directory")?;
    
    Ok(pictures_dir.join("AI Formula Scanner"))
}

/// 关闭所有遮罩窗口
#[tauri::command]
pub async fn close_all_overlays(app: AppHandle) -> Result<(), String> {
    let displays = get_displays()?;

    for display in displays {
        let label = format!("snip-overlay-{}", display.index);
        if let Some(window) = app.get_window(&label) {
            let _ = window.close();
        }
    }

    Ok(())
}

/// 开始从区域截图进行识别
#[tauri::command]
pub async fn start_recognition_from_region_capture(app: AppHandle, image_path: String) -> Result<(), String> {
    // 获取主窗口
    if let Some(main_window) = app.get_window("main") {
        // 发送事件到主窗口，通知开始识别
        main_window.emit("region-capture-completed", image_path)
            .map_err(|e| format!("Failed to emit event: {}", e))?;
    }

    Ok(())
}
