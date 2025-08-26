import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';

export default defineConfig(async () => ({
	plugins: [
		sveltekit()
	],

	// Vite options tailored for Tauri development
	//
	// 1. prevent vite from obscuring rust errors
	clearScreen: false,
	// 2. 配置服务器默认端口，但允许自动查找可用端口
	server: {
		port: 5173, // 使用Tauri默认端口作为首选
		strictPort: false, // 允许在端口被占用时自动寻找下一个可用端口
		watch: {
			// 3. tell vite to ignore watching `src-tauri`
			ignored: ['**/src-tauri/**']
		}
	},
	// 优化依赖处理，减少兼容性警告
	optimizeDeps: {
		include: ['mathjax', 'katex'],
		exclude: ['@tauri-apps/api']
	},
	// 定义全局变量以避免某些库的错误
	define: {
		global: 'globalThis'
	}
}));
