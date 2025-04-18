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
```
