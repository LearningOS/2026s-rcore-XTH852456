# lab2 mmap/munmap & trace/get_time

## 修改概述
- 重写 `sys_get_time`：基于当前任务页表安全写回 `TimeVal`，支持跨页写入。
- 重写 `sys_trace`：
  - `trace_request=0` 安全读用户地址 1 字节；无映射/不可读/非用户页返回 `-1`。
  - `trace_request=1` 安全写用户地址 1 字节；无映射/不可写/非用户页返回 `-1`。
  - `trace_request=2` 返回当前任务 syscall 计数，本次调用计入统计。
- 实现 `sys_mmap/sys_munmap`：
  - `mmap` 检查页对齐、`prot` 合法性、地址溢出、映射冲突。
  - `munmap` 检查页对齐、地址溢出、目标区间必须已映射。
- 增加任务级 syscall 计数结构和接口。
- 增加带权限检查的用户地址翻译函数。

## 关键实现点
- `os/src/mm/page_table.rs`
  - 新增 `translated_byte_buffer_checked(...)`，按页检查 `PTE_U` 与 `R/W` 权限。
  - 修复 `translate(...)` 仅对有效 PTE 返回 `Some`。
- `os/src/mm/memory_set.rs`
  - 新增 `mmap(...)` 与 `munmap(...)` 供系统调用使用。
- `os/src/task/task.rs`
  - 在 `TaskControlBlock` 中新增 syscall 计数字段。
- `os/src/task/mod.rs`
  - 新增当前任务 syscall 计数与 `mmap/munmap` 封装接口。
- `os/src/syscall/mod.rs`
  - syscall 总入口统一计数。
- `os/src/syscall/process.rs`
  - 完整实现 `sys_get_time/sys_trace/sys_mmap/sys_munmap`。

## 测试结果
- `make run CHAPTER=4 TEST=4 BASE=0`：通过，关键输出包含 `Test 04_1/04_4/04_5/04_6` 与 `Test trace_1 OK!`。
- `cd ci-user && make test CHAPTER=4`：功能检查 `16/16` 通过。
