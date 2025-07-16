## 1. 安装

```
npm i maya-cli-rs -g
```

> 注意：暂时只支持 windows 系统

## 2. 使用

```bash

maya -c n # 清除node_modules
maya -c lock # 清除package-lock.json, yarn.lock, pnpm-lock.yaml
maya -g m # 会在当前目录下面执行 git add . && git commit -m 'update' && git push
maya -p a # 会检测vite的outDir配置，然后把对应的文件夹打包成zip
maya -p g # 会忽略当前目录下的.gitignore文件中定义的文件和文件夹，把当前目录下其它所有的文件和文件夹打包成zip文件
```

```bash
maya -o all # 会把当前目录下面png，jpg，jpeg这三种格式的所以的图片，在保证质量的前提下压缩体积，默认复写模式
maya -o n all # 添加 n，从复写模式改成新文件模式
maya -o png # 只压缩png图片
maya -o jpg # 只压缩jpg图片
maya -o jpeg # 只压缩jpeg图片
```

```bash
maya -t mp4 m3u8 # 会把当前目录下面所有的mp4视频转换成m3u8格式，转换后的文件会放在以原视频名称命名的文件夹中
```

## 3. 特性

- ✅ **自动 FFmpeg 下载**: 首次使用时会自动下载 FFmpeg，无需手动安装
- ✅ **实时进度显示**: 下载和转换过程都有详细的百分比进度条
- ✅ **智能输出**: 转换后的文件自动放在以原视频名称命名的文件夹中
- ✅ **批量处理**: 支持同时转换多个 mp4 文件
