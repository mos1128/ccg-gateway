# 发版指南

## 发布新版本


```bash
# 1. 确保代码已提交并推送
git add .
git commit -m "feat: 你的提交信息"
git push

# 2. 创建带信息的标签并推送（标签信息会作为 Release 说明）
git tag -a v1.2.0 -m "新增xxx功能，修复xxx问题"
git push origin v1.2.0

# 3. 等待 GitHub Actions 自动构建并发布（约 15-20 分钟）
```

## 发版错误：删除标签

如果发版有误，需要同时删除本地和远程标签，以及对应的 Release：

```bash
# 删除本地标签
git tag -d v1.2.0

# 删除远程标签（会自动取消正在运行的 workflow）
git push origin --delete v1.2.0

# 然后去 GitHub Releases 页面手动删除对应的 Release（如果已创建）
```

删除后修正代码，重新走发布流程即可。
