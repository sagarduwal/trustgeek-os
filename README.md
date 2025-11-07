
```bash
cargo build --release

cargo espflash flash --release --port /dev/ttyUSB0 --target xtensa-esp32-none-elf

cargo espflash monitor --port /dev/ttyUSB0

```


groups
sudo usermod -aG dialout "$USER" && newgrp dialout
