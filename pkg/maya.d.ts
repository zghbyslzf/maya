/* tslint:disable */
/* eslint-disable */
/**
 * 清除目录中的锁文件 (package-lock.json, yarn.lock 等)
 */
export function clear_lock_files(dir: string): number;
/**
 * 清除目录中的所有 node_modules 文件夹
 */
export function clear_node_modules(dir: string): number;
