use core::cell::RefCell;
use core::net::Ipv4Addr;

use cyw43::Control;
use cyw43::NetDriver;
use cyw43_pio::PioSpi;
use embassy_executor::Spawner;
use embassy_net::device::DriverAdapter;
use embassy_net::driver::Driver;
use embassy_net::udp::PacketMetadata;
use embassy_net::{driver, Inner, SocketStack, LOCAL_PORT_MAX, LOCAL_PORT_MIN};
use embassy_net::{
    to_smoltcp_hardware_address, Config, ConfigV4, IpCidr, Ipv4Address, Stack, StackResources,
};
use embassy_rp::gpio::Level;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::DMA_CH0;
use embassy_rp::peripherals::PIN_23;
use embassy_rp::peripherals::PIN_24;
use embassy_rp::peripherals::PIN_25;
use embassy_rp::peripherals::PIN_29;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::Pio;
use embassy_sync::waitqueue::WakerRegistration;
use embassy_time::Instant;
use smoltcp::iface::{Interface, SocketSet};
use smoltcp::phy::Medium;
use smoltcp::wire::HardwareAddress;
use smoltcp::time::Instant as SmolInstant;

use embassy_rp::Peripherals;
use heapless::Vec;
use log::info;
use rand::Rng;
use static_cell::make_static;

use crate::Irqs;
use crate::WEB_TASK_POOL_SIZE;

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static embassy_net::Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

pub fn new<D, const SOCK: usize>(
    mut device: D,
    config: Config,
    resources: &'static mut StackResources<SOCK>,
    random_seed: u64,
    outside_ip: Ipv4Address,
) -> Stack<D>
where
    D: Driver,
{
    let (hardware_addr, medium) = to_smoltcp_hardware_address(device.hardware_address());
    let mut iface_cfg = smoltcp::iface::Config::new(hardware_addr);
    iface_cfg.random_seed = random_seed;

    let mut iface = Interface::new(
        iface_cfg,
        &mut DriverAdapter {
            inner: &mut device,
            cx: None,
            medium,
        },
        SmolInstant::from_micros(Instant::now().as_micros() as i64),
    );

    iface.update_ip_addrs(|v| {
        v.push(IpCidr::new(smoltcp::wire::IpAddress::Ipv4(outside_ip), 24));
    });

    let sockets = SocketSet::new(&mut resources.sockets[..]);

    let next_local_port =
        (random_seed % (LOCAL_PORT_MAX - LOCAL_PORT_MIN) as u64) as u16 + LOCAL_PORT_MIN;

    let mut socket = SocketStack {
        sockets,
        iface,
        waker: WakerRegistration::new(),
        next_local_port,
    };

    let mut inner = Inner {
        device,
        link_up: false,
        static_v4: None,
        config_waker: WakerRegistration::new(),
    };

    inner.set_config_v4(&mut socket, config.ipv4);

    inner.apply_static_config(&mut socket);
    Stack {
        socket: RefCell::new(socket),
        inner: RefCell::new(inner),
    }
}

pub async fn set_up_network_stack(
    spawner: &Spawner,
    power_pin: PIN_23,
    cs_pin: PIN_25,
    pio_0: PIO0,
    dio: PIN_24,
    clk: PIN_29,
    dma: DMA_CH0,
    server_ip_address: embassy_net::Ipv4Address,
    outside_ip_address: embassy_net::Ipv4Address,
) -> (Control<'static>, &'static Stack<NetDriver<'static>>) {
    let fw = include_bytes!("../firmware/43439A0.bin");
    let clm = include_bytes!("../firmware/43439A0_clm.bin");

    let pwr = Output::new(power_pin, Level::Low);
    let cs = Output::new(cs_pin, Level::High);
    let mut pio_wifi = Pio::new(pio_0, Irqs);
    let spi = cyw43_pio::PioSpi::new(
        &mut pio_wifi.common,
        pio_wifi.sm0,
        pio_wifi.irq0,
        cs,
        dio,
        clk,
        dma,
    );

    let state = make_static!(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    spawner.must_spawn(wifi_task(runner));
    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;
    let stack = &*make_static!(new(
        net_device,
        embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
            address: embassy_net::Ipv4Cidr::new(server_ip_address, 24),
            gateway: Some(server_ip_address),
            dns_servers: Vec::from_slice(&[server_ip_address]).unwrap(),
        }),
        make_static!(embassy_net::StackResources::<WEB_TASK_POOL_SIZE>::new()),
        embassy_rp::clocks::RoscRng.gen(),
        outside_ip_address
    ));

    spawner.must_spawn(net_task(stack));
    stack.wait_config_up().await;

    info!("Starting access point...");

    control.start_ap_open("pico", 5).await;

    (control, stack)
}
