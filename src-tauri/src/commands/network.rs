use crate::models::{AppError, NetworkAdapter};

/// Cross-platform entry point. Real enumeration only exists on
/// Windows via `GetAdaptersAddresses`; elsewhere returns a structured
/// "not supported" error rather than fake adapters.
#[tauri::command]
pub fn list_network_adapters() -> Result<Vec<NetworkAdapter>, AppError> {
    #[cfg(windows)]
    {
        windows_impl::enumerate_adapters()
    }
    #[cfg(not(windows))]
    {
        Err(AppError::not_supported("Network adapter enumeration"))
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
    use windows::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_INCLUDE_PREFIX,
        IF_TYPE_ETHERNET_CSMACD, IF_TYPE_IEEE80211, IF_TYPE_PPP, IF_TYPE_SOFTWARE_LOOPBACK,
        IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::Networking::WinSock::{AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6};

    pub fn enumerate_adapters() -> Result<Vec<NetworkAdapter>, AppError> {
        unsafe {
            let mut size: u32 = 15_000; // typical starting size, grown on overflow
            let mut buffer: Vec<u8>;
            loop {
                buffer = vec![0u8; size as usize];
                let result = GetAdaptersAddresses(
                    AF_UNSPEC.0 as u32,
                    GAA_FLAG_INCLUDE_PREFIX | GAA_FLAG_INCLUDE_GATEWAYS,
                    None,
                    Some(buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
                    &mut size,
                );
                if result == ERROR_SUCCESS.0 {
                    break;
                } else if result == ERROR_BUFFER_OVERFLOW.0 {
                    continue; // size was updated; retry with larger buffer
                } else {
                    return Err(AppError {
                        code: "ADAPTER_ENUM_FAILED".into(),
                        message: "Failed to enumerate network adapters.".into(),
                        details: Some(format!("Win32 error code {result}")),
                        recoverable: true,
                    });
                }
            }

            let mut adapters = Vec::new();
            let mut current = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;

            while !current.is_null() {
                let adapter = &*current;
                adapters.push(parse_adapter(adapter));
                current = adapter.Next;
            }

            Ok(adapters)
        }
    }

    unsafe fn parse_adapter(adapter: &IP_ADAPTER_ADDRESSES_LH) -> NetworkAdapter {
        let name = wide_to_string(adapter.FriendlyName.0);
        let description = wide_to_string(adapter.Description.0);

        let adapter_type = match adapter.IfType {
            t if t == IF_TYPE_ETHERNET_CSMACD => "Ethernet",
            t if t == IF_TYPE_IEEE80211 => "Wi-Fi",
            t if t == IF_TYPE_PPP => "VPN",
            t if t == IF_TYPE_SOFTWARE_LOOPBACK => "Loopback",
            _ => "Other",
        }
        .to_string();

        // OperStatus: 1 = Up, 2 = Down, others = various transitional/unknown states.
        let status = match adapter.OperStatus.0 {
            1 => "Up",
            2 => "Down",
            _ => "Unknown",
        }
        .to_string();

        let mac_address = if adapter.PhysicalAddressLength > 0 {
            let bytes = &adapter.PhysicalAddress[..adapter.PhysicalAddressLength as usize];
            Some(
                bytes
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<Vec<_>>()
                    .join(":"),
            )
        } else {
            None
        };

        let mut ipv4_addresses = Vec::new();
        let mut ipv6_addresses = Vec::new();
        let mut unicast = adapter.FirstUnicastAddress;
        while !unicast.is_null() {
            let addr = &*unicast;
            if let Some(sockaddr) = addr.Address.lpSockaddr.as_ref() {
                match (*sockaddr).sa_family {
                    windows::Win32::Networking::WinSock::AF_INET => {
                        let sin = &*(sockaddr as *const _ as *const SOCKADDR_IN);
                        let b = sin.sin_addr.S_un.S_addr.to_le_bytes();
                        ipv4_addresses.push(format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]));
                    }
                    windows::Win32::Networking::WinSock::AF_INET6 => {
                        let sin6 = &*(sockaddr as *const _ as *const SOCKADDR_IN6);
                        let segs = sin6.sin6_addr.u.Word;
                        let parts: Vec<String> =
                            segs.iter().map(|s| format!("{:x}", u16::from_be(*s))).collect();
                        ipv6_addresses.push(parts.join(":"));
                    }
                    _ => {}
                }
            }
            unicast = addr.Next;
        }

        let mut gateway = None;
        let mut gw_ptr = adapter.FirstGatewayAddress;
        if !gw_ptr.is_null() {
            let gw = &*gw_ptr;
            if let Some(sockaddr) = gw.Address.lpSockaddr.as_ref() {
                if (*sockaddr).sa_family == windows::Win32::Networking::WinSock::AF_INET {
                    let sin = &*(sockaddr as *const _ as *const SOCKADDR_IN);
                    let b = sin.sin_addr.S_un.S_addr.to_le_bytes();
                    gateway = Some(format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]));
                }
            }
        }
        let _ = &mut gw_ptr; // silence unused-mut if the loop above is later extended

        let mut dns_servers = Vec::new();
        let mut dns_ptr = adapter.FirstDnsServerAddress;
        while !dns_ptr.is_null() {
            let dns = &*dns_ptr;
            if let Some(sockaddr) = dns.Address.lpSockaddr.as_ref() {
                if (*sockaddr).sa_family == windows::Win32::Networking::WinSock::AF_INET {
                    let sin = &*(sockaddr as *const _ as *const SOCKADDR_IN);
                    let b = sin.sin_addr.S_un.S_addr.to_le_bytes();
                    dns_servers.push(format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]));
                }
            }
            dns_ptr = dns.Next;
        }

        NetworkAdapter {
            name,
            description,
            adapter_type,
            status,
            mac_address,
            ipv4_addresses,
            ipv6_addresses,
            gateway,
            dns_servers,
            dhcp_enabled: Some(adapter.Flags & 0x0004 != 0), // IP_ADAPTER_DHCP_ENABLED
            link_speed_mbps: if adapter.TransmitLinkSpeed > 0 {
                Some(adapter.TransmitLinkSpeed / 1_000_000)
            } else {
                None
            },
        }
    }

    unsafe fn wide_to_string(ptr: *const u16) -> String {
        if ptr.is_null() {
            return String::new();
        }
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        String::from_utf16_lossy(slice)
    }
}
