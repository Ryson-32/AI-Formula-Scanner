<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/tauri';
  import { appWindow } from '@tauri-apps/api/window';

  let dragging = false;
  let start = { x: 0, y: 0 };
  let current = { x: 0, y: 0 };
  let scaleFactor = 1;
  let displayIndex = 0;
  let overlayElement: HTMLDivElement;

  onMount(() => {

    // 监听键盘事件
    const handleKeyDown = async (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        await closeOverlay();
      }
    };

    // 异步初始化
    (async () => {
      try {
        // 获取缩放因子
        scaleFactor = await appWindow.scaleFactor();

        // 从 URL 读取显示器序号
        const urlParams = new URLSearchParams(window.location.search);
        const indexParam = urlParams.get('i');
        displayIndex = indexParam ? parseInt(indexParam, 10) : 0;

        // 聚焦窗口以确保能接收键盘事件
        await appWindow.setFocus();

      } catch (error) {
        
      }
    })();

    window.addEventListener('keydown', handleKeyDown);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  });

  async function closeOverlay() {
    try {
      await invoke('close_all_overlays');
    } catch (error) {
      
      // 如果调用失败，直接关闭当前窗口
      await appWindow.close();
    }
  }

  async function notifyMainWindowToRecognize(imagePath: string) {
    try {
      // 通过Tauri的事件系统通知主窗口
      await invoke('start_recognition_from_region_capture', {
        imagePath: imagePath
      });
      
    } catch (error) {
      
    }
  }

  function handleMouseDown(e: MouseEvent) {
    dragging = true;
    start = { x: e.clientX, y: e.clientY };
    current = { ...start };
    
    // 阻止默认行为
    e.preventDefault();
  }

  function handleMouseMove(e: MouseEvent) {
    if (dragging) {
      current = { x: e.clientX, y: e.clientY };
    }
  }

  async function handleMouseUp(_e: MouseEvent) {
    if (!dragging) return;

    dragging = false;

    // 计算选择区域
    const x = Math.min(start.x, current.x);
    const y = Math.min(start.y, current.y);
    const width = Math.abs(start.x - current.x);
    const height = Math.abs(start.y - current.y);

    

    // 如果选择区域太小，取消操作
    if (width < 10 || height < 10) {
      await closeOverlay();
      return;
    }

    try {
      // 获取窗口位置
      const position = await appWindow.outerPosition();

      // 调用后端完成截图
      const imagePath = await invoke<string>('complete_capture', {
        args: {
          rect: [x, y, width, height],
          overlay_pos: [position.x, position.y],
          scale_factor: scaleFactor,
          display_index: displayIndex,
        }
      });

      // 通知主窗口开始识别
      await notifyMainWindowToRecognize(imagePath);

      // 关闭遮罩窗口
      await closeOverlay();

    } catch (error) {
      
      await closeOverlay();
    }
  }

  // 计算选择矩形的样式
  $: rectStyle = dragging ? {
    left: `${Math.min(start.x, current.x)}px`,
    top: `${Math.min(start.y, current.y)}px`,
    width: `${Math.abs(start.x - current.x)}px`,
    height: `${Math.abs(start.y - current.y)}px`,
  } : {};
</script>

<div 
  class="overlay"
  bind:this={overlayElement}
  on:mousedown={handleMouseDown}
  on:mousemove={handleMouseMove}
  on:mouseup={handleMouseUp}
  role="button"
  tabindex="0"
>
  {#if dragging}
    <div 
      class="selection-rect"
      style="left: {rectStyle.left}; top: {rectStyle.top}; width: {rectStyle.width}; height: {rectStyle.height};"
    ></div>
  {/if}
  
  <div class="instructions">
    <p>拖拽选择要识别的区域</p>
    <p class="hint">按 ESC 取消</p>
  </div>
</div>

<style>
  /* 确保整个页面透明 */
  :global(html), :global(body) {
    background: transparent !important;
  }

  .overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: transparent;  /* 完全透明的遮罩 */
    cursor: crosshair;
    user-select: none;
    z-index: 9999;
  }

  .selection-rect {
    position: absolute;
    border: 2px solid #007acc;
    background: rgba(0, 122, 204, 0.1);
    box-shadow:
      0 0 0 1px rgba(255, 255, 255, 0.9),
      0 0 8px rgba(0, 122, 204, 0.5);  /* 只在选择框周围添加蓝色阴影 */
    pointer-events: none;
  }

  .instructions {
    position: absolute;
    top: 20px;
    left: 50%;
    transform: translateX(-50%);
    color: white;
    text-align: center;
    font-size: 16px;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.8);
    pointer-events: none;
    z-index: 10000;
  }

  .instructions p {
    margin: 0;
    padding: 4px 0;
  }

  .hint {
    font-size: 14px;
    opacity: 0.8;
  }

  /* 确保在拖拽时光标保持为十字 */
  .overlay:active {
    cursor: crosshair;
  }
</style>
