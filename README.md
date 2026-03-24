# rCore 教程代码

## 代码

- [实验源码](https://github.com/LearningOS/rCore-Tutorial-Code)

## 文档

- 精简手册：[rCore-Tutorial-Guide](https://LearningOS.github.io/rCore-Tutorial-Guide/)

- 详细教程书：[rCore-Tutorial-Book-v3](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)

## rCore 教程代码的 OS API 文档

- [ch1 的 OS API 文档](https://learningos.github.io/rCore-Tutorial-Code/ch1/os/index.html)
  和 [ch2 的 OS API 文档](https://learningos.github.io/rCore-Tutorial-Code/ch2/os/index.html)
- [ch3 的 OS API 文档](https://learningos.github.io/rCore-Tutorial-Code/ch3/os/index.html)
  和 [ch4 的 OS API 文档](https://learningos.github.io/rCore-Tutorial-Code/ch4/os/index.html)
- [ch5 的 OS API 文档](https://learningos.github.io/rCore-Tutorial-Code/ch5/os/index.html)
  和 [ch6 的 OS API 文档](https://learningos.github.io/rCore-Tutorial-Code/ch6/os/index.html)
- [ch7 的 OS API 文档](https://learningos.github.io/rCore-Tutorial-Code/ch7/os/index.html)
  和 [ch8 的 OS API 文档](https://learningos.github.io/rCore-Tutorial-Code/ch8/os/index.html)
- [ch9 的 OS API 文档](https://learningos.github.io/rCore-Tutorial-Code/ch9/os/index.html)

## 相关资源

- [学习资源](https://github.com/LearningOS/rust-based-os-comp2025/blob/main/relatedinfo.md)

## 环境准备

```bash
$ git clone https://github.com/LearningOS/2026s-rcore-[YOUR_USER_NAME].git
$ cd 2026s-rcore-[YOUR_USER_NAME]
```

## 编译与运行

```bash
# 先准备编译和运行环境
$ git clone https://github.com/LearningOS/rCore-Tutorial-Test.git user
$ git checkout ch$ID
$ cd os
# 运行 ch$ID 对应的 OS
$ make run
```

如果你想使用 Docker 进行编译和运行，可以使用以下命令：

```bash
# 将 `rCore-Tutorial-Test` 仓库克隆到本地后，可使用以下命令编译并运行：
$ make build_docker
$ make docker
```

如果你在 Docker 中访问 GitHub 等境外资源时遇到网络问题，可以按以下阶段处理：

- Docker pull：
  1. 使用代理：https://docs.docker.com/reference/cli/docker/image/pull/#proxy-configuration
  2. 使用可用的国内镜像源（请自行检索）

- Docker build：使用代理 https://docs.docker.com/engine/cli/proxy/#build-with-a-proxy-configuration

- Docker run：使用代理选项，相关操作与 `Docker build` 类似，可自行查阅相关资料

注意：`$ID` 的取值范围是 `[1-9]`

## 评分

```bash
# 先准备编译和运行环境
$ rm -rf ci-user
$ git clone https://github.com/LearningOS/rCore-Tutorial-Checker.git ci-user
$ git clone https://github.com/LearningOS/rCore-Tutorial-Test.git ci-user/user
$ git checkout ch$ID
# 在 ch$ID 上进行更多测试并评分
$ cd ci-user && make test CHAPTER=$ID
```

注意：`$ID` 的取值范围是 `[3,4,5,6,8]`
