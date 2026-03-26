# ch3 `sys_trace` 实验报告

## 1. 实验目标
在 ch3 已支持多任务分时调度的基础上，实现新的系统调用：

- 接口：`fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize`
- 系统调用号：`410`

并满足三种行为：

1. `trace_request = 0`：读取当前任务地址 `id` 处 1 字节数据并返回。
2. `trace_request = 1`：向当前任务地址 `id` 写入 `data as u8`，返回 `0`。
3. `trace_request = 2`：查询当前任务系统调用编号为 `id` 的调用次数，且本次 `sys_trace` 也要计入统计。
4. 其他 `trace_request` 返回 `-1`。

## 2. 需求分析与设计思路

### 2.1 关键需求
- 需要按“任务”维度统计系统调用次数，而不是全局统计。
- `trace_request = 2` 时，本次 `sys_trace` 调用本身也要被统计。
- 本章不要求地址安全检查，允许直接读写用户地址。

### 2.2 设计决策
为保证统计逻辑统一、且不遗漏任何系统调用，采用如下方案：

- 在内核系统调用总入口 `syscall()` 中统一计数。
- 每个任务维护一个独立的 syscall 计数数组。
- `sys_trace(request=2)` 仅做查询，不在 `sys_trace` 内重复计数。

这样可确保：
- 任何 syscall 都会被自动统计；
- 查询 `SYSCALL_TRACE(410)` 时，本次调用一定已经被计入。

## 3. 具体实现

### 3.1 数据结构扩展（按任务统计）
文件：`os/src/task/task.rs`

- 新增常量：`MAX_SYSCALL_NUM: usize = 512`
- 在 `TaskControlBlock` 中新增字段：
  - `syscall_times: [usize; MAX_SYSCALL_NUM]`

作用：为每个任务保存独立的 syscall 计数表，索引即 syscall id。

### 3.2 任务初始化与访问接口
文件：`os/src/task/mod.rs`

- 在任务数组初始化时，将 `syscall_times` 全部置零。
- 新增接口：
  - `record_current_syscall(syscall_id: usize)`：给当前运行任务对应 syscall 计数加一。
  - `current_task_syscall_count(syscall_id: usize) -> usize`：读取当前任务某个 syscall 的计数。
- 边界处理：当 `syscall_id >= MAX_SYSCALL_NUM` 时，写入忽略、查询返回 `0`，避免越界。

### 3.3 在 syscall 总入口统一计数
文件：`os/src/syscall/mod.rs`

在 `pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize` 的 `match` 分发前增加：

```rust
record_current_syscall(syscall_id);
```

这一步是本次实现的核心：
- 统一入口计数，避免分散在各个 `sys_xxx` 中导致遗漏；
- 满足“本次 `sys_trace` 也计入统计”的要求。

### 3.4 实现 `sys_trace` 三种语义
文件：`os/src/syscall/process.rs`

实现如下逻辑：

- `trace_request == 0`：`id` 按 `*const u8` 解引用并返回。
- `trace_request == 1`：`id` 按 `*mut u8` 写入 `data as u8`，返回 `0`。
- `trace_request == 2`：返回 `current_task_syscall_count(id) as isize`。
- 其他：返回 `-1`。

## 4. 关键正确性说明

### 4.1 为什么本次 `sys_trace` 会被计入？
调用路径是：

用户态 `ecall` -> 内核 `syscall()` 总入口 -> `record_current_syscall(410)` -> 分发到 `sys_trace()`。

因此在 `sys_trace(request=2)` 查询时，当前这次 `sys_trace` 已经先被统计，满足题目要求。

### 4.2 为什么使用“每任务计数”而不是全局计数？
题目要求追踪“当前任务”的系统调用历史。使用 TCB 内部计数数组可天然保证任务间互不干扰。

## 5. 测试与验证
执行命令：

```bash
cd os
make run CHAPTER=3 TEST=3 BASE=0
```

该命令会构建并运行 ch3 普通测试（包含 `ch3_trace`）。

关键输出包含：
- `Test trace OK!`
- `Test sleep1 passed!`
- `Test sleep OK!`

说明：`sys_trace` 的读/写/计数语义均通过当前实验用例验证。

## 6. 结果总结
本次修改完成了 ch3 对 `sys_trace(410)` 的支持，核心点是：

- 在 syscall 总入口统一计数；
- 将计数数据放入任务控制块实现“按任务追踪”；
- 按题目定义实现 `sys_trace` 三种请求语义；
- 通过 `ch3_trace` 用例验证功能正确。

后续在实现地址空间后，可进一步为读写地址行为增加合法性检查与隔离机制。