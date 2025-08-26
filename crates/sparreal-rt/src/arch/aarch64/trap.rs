#[somehal::irq_handler]
fn irq_handler() {
    sparreal_kernel::irq::handle_irq();
}
