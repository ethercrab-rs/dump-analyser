# Dell tool - doesn't work on Linux Mint

My card is a Dell i350-T4 so I got the update from
<https://www.dell.com/support/home/en-my/drivers/driversdetails?driverid=wttp6>

Original FW

```
❯ sudo ethtool -i enp1s0f0
driver: igb
version: 5.15.0-1032-realtime
firmware-version: 1.67, 0x80000e93, 17.5.10
expansion-rom-version:
bus-info: 0000:01:00.0
supports-statistics: yes
supports-test: yes
supports-eeprom-access: yes
supports-register-dump: yes
supports-priv-flags: yes
```

Doesn't work! Maybe the installer doesn't like my kernel/hardware/OS.

# Using `BootUtil`

Download here
<https://www.intel.com/content/www/us/en/support/articles/000005790/software/manageability-products.html>
(also a manual)

Following some of this <https://calvin.me/how-to-update-intel-nic-firmware/> but from the Linux CLI
instead of UEFI.

And this guide <https://www.reddit.com/r/homelab/comments/eda5qp/updating_firmware_on_i350_t4/>

Extract the
[files](https://www.intel.com/content/www/us/en/download/15755/intel-ethernet-connections-boot-utility-preboot-images-and-efi-drivers.html?),
then:

```bash
cd BootUtil/Linux_x64
chmod +x ./bootutil64e
sudo ./bootutil64e -up=combo -all

reboot
```
