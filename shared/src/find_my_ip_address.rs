use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Helper method to find local IP address, if possible
pub fn find_my_ip_address() -> Option<IpAddr> {
    let ip = local_ipaddress::get().unwrap_or_default();

    if let Ok(addr) = ip.parse::<Ipv4Addr>() {
        return Some(IpAddr::V4(addr));
    } else if let Ok(addr) = ip.parse::<Ipv6Addr>() {
        return Some(IpAddr::V6(addr));
    } else {
        return None;
    }
}
