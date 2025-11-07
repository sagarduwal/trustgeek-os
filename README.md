
```bash
cargo build --release

cargo espflash flash --release --port /dev/ttyUSB0 --target xtensa-esp32-none-elf

cargo espflash monitor --port /dev/ttyUSB0

```




# see device perms and your groups
ls -l /dev/ttyUSB0
groups

# add yourself to the serial group (usually 'dialout' on Debian/Ubuntu)
sudo usermod -aG dialout "$USER"

# apply without reboot (new shell only), then confirm 'dialout' is listed
newgrp dialout
groups
