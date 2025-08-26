use somehal::{BootInfo, mem::phys_to_virt};
use sparreal_kernel::{globals::PlatformInfoKind, hal_al};

use crate::mem;

use super::debug;

#[somehal::entry]
fn main(args: &BootInfo) -> ! {
    if let Some(fdt) = args.fdt {
        somehal::println!("FDT at {:p}", fdt.as_ptr());
    }
    debug::setup_by_fdt(args.fdt, phys_to_virt);
    mem::setup_boot_args(args);
    hal_al::run::run(PlatformInfoKind::new_fdt(
        args.fdt.map_or(0, |fdt| fdt.as_ptr() as usize).into(),
    ));
}
