#![no_std]
#![no_main]
#![feature(used_with_arg)]

#[bare_test::tests]
mod tests {

    use bare_test::*;
    use globals::{PlatformInfoKind, global_val};
    use simple_test::new_dma;

    #[test]
    fn test2() {
        let _fdt = match &global_val().platform_info {
            PlatformInfoKind::DeviceTree(fdt) => fdt.get(),
        };

        let data = new_dma();

        println!("test2: data len: {}", data.len());
        assert!(!data.is_empty(), "DMA vector should not be empty");
        let ptr = data.as_ptr();
        println!("test2: data ptr: {ptr:#p}");

        let phys = data.bus_addr();

        println!("test2: data phys: {phys:#x}");
    }
}
