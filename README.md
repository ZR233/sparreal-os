# 雀实操作系统 Sparreal

麻雀虽小，五脏俱全的实时操作系统。

## 环境搭建

```shell
git clone --recurse-submodules  git@github.com:qclic/sparreal-os.git
```

1. 安装 Rust
2. 安装依赖

```bash
cargo install ostool
```

### Windows

安装[Qemu](https://www.qemu.org/download/#windows), 并加入环境变量。

### Linux

安装[Qemu](https://www.qemu.org/download/#linux)

### Mac

安装[Qemu](https://www.qemu.org/download/#macos)

## 构建

```bash
ostool build
```

## Qemu 测试

```bash
ostool run qemu
```

## Qemu + Vscode 断点调试

```bash
ostool run qemu -d
```

vscode 选择调试配置 `KDebug`， 点击 `Run and Debug` 按钮。

## U-Boot 调试

需要连接开发板串口。

```bash
ostool run uboot
```

## 配置

首次执行 `ostool` 任务后，会在根目录生成默认配置文件 `.project.toml`。

## 平台适配

 1. 实现平台接口
 2. 初始化页表，启动MMU。
 3. 启动内核

```rust
use sparreal_kernel::platform::Platform;
use sparreal_macros::api_impl;

pub struct PlatformImpl;

// 实现接口
#[api_impl]
impl Platform for PlatformImpl {
    unsafe fn wait_for_interrupt() {
        aarch64_cpu::asm::wfi();
    }
    ... other fn
}
```

```rust
pub use sparreal_kernel::*;

// 启动MMU，并进入函数
unsafe fn boot(kconfig: kernel::KernelConfig) -> ! {
    // 初始化日志和内存分配器。
    kernel::init_log_and_memory(&kconfig);
    // 注册驱动
    kernel::driver_register_append(drivers::registers());
    // 启动内核
    kernel::run()
}
``
