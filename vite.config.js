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
	}
}));
