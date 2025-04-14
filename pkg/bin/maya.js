#!/usr/bin/env node

const maya = require('../maya');
const path = require('path');
const process = require('process');

function printHelp() {
    console.log(`
使用方法: maya [选项] [目录]

选项:
  -c, --clear <type>  清除指定类型的文件
                      n: 清除 node_modules
                      l: 清除锁文件
  -h, --help         显示帮助信息

示例:
  maya -c n ./project  # 清除指定目录下的 node_modules
  maya -c l ./project  # 清除指定目录下的锁文件
`);
}

function main() {
    const args = process.argv.slice(2);
    const targetDir = args[args.length - 1];
    
    // 检查是否是帮助请求
    if (args.includes('-h') || args.includes('--help')) {
        printHelp();
        return;
    }

    // 检查清除选项
    const clearIndex = args.findIndex(arg => arg === '-c' || arg === '--clear');
    if (clearIndex === -1) {
        console.error('错误: 缺少清除选项');
        printHelp();
        process.exit(1);
    }

    const clearType = args[clearIndex + 1];
    if (!clearType) {
        console.error('错误: 缺少清除类型');
        printHelp();
        process.exit(1);
    }

    // 检查目标目录
    if (!targetDir || targetDir.startsWith('-')) {
        console.error('错误: 缺少目标目录');
        printHelp();
        process.exit(1);
    }

    try {
        let count;
        switch (clearType) {
            case 'n':
                count = maya.clear_node_modules(targetDir);
                console.log(`已清除 ${count} 个 node_modules 文件夹`);
                break;
            case 'l':
                count = maya.clear_lock_files(targetDir);
                console.log(`已清除 ${count} 个锁文件`);
                break;
            default:
                console.error('错误: 无效的清除类型');
                printHelp();
                process.exit(1);
        }
    } catch (error) {
        console.error('错误:', error.message);
        process.exit(1);
    }
}

main(); 