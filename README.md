## 1. 安装

```
npm i maya-cli-rs -g
```

> 注意：暂时只支持 windows 系统

## 2. 使用

```bash

# 完整形式
maya clean --types n     # 清除node_modules
maya clean -t n          # 同上（-t 是 --types 的短参数）
maya c -t n              # 同上（c 是 clean 的别名）

# 短个别名形式（等效）
maya c -t n          # 清除node_modules
maya c -t lock       # 清除lock文件
maya g -o m          # git add . && git commit && git push
maya p -t a          # 会检测vite的outDir配置，然后把对应的文件夹打包成zip
maya p -t g          # 会忽略当前目录下的.gitignore文件中定义的文件和文件夹，把当前目录下其它所有的文件和文件夹打包成zip文件
```

```bash
maya optimize -t all    # 压缩所有png/jpg/jpeg，默认覆写
maya o -t n all        # 添加 n → 新文件模式
maya o -t png          # 只压缩png
maya o -t jpg          # 只压缩jpg
```

```bash
maya transform -t mp4 m3u8  # mp4视频转m3u8
maya t -t mp4 m3u8           # 同上
```

## 3. 特性

- ✅ **自动 FFmpeg 下载**: 首次使用时会自动下载 FFmpeg，无需手动安装
- ✅ **实时进度显示**: 下载和转换过程都有详细的百分比进度条
- ✅ **智能输出**: 转换后的文件自动放在以原视频名称命名的文件夹中
- ✅ **批量处理**: 支持同时转换多个 mp4 文件
