# MCF hardware support request

Written by `mcf support`. Nothing was sent: this is a file for you to read and send if you choose.

It carries driver names, sensor labels and readings, and the paths MCF looked in. It carries no prompt, no model output, no file content and no path from a model directory.

## What MCF is

- build: MCF 0.1.0-m0  (revision unknown, rustc 1.98.0 (88d9e12ae 2026-08-18), target x86_64-unknown-linux-gnu, profile release)

## What this machine is

- architecture: x86_64
- operating system: linux
- kernel: 6.18.35+rex+2-amd64

## Temperature sensors

Read from `/sys/class/hwmon/*`.

- k10temp/Tctl 48.1 °C (processor package), critical point not published by this chip
- amdgpu/edge 29.0 °C (accelerator), critical point not published by this chip
- nvme/Composite 30.8 °C (storage), critical at 87.8 °C
- nvme/Sensor 1 30.8 °C (storage), critical point not published by this chip
- acpitz/unlabelled 30.8 °C (board), critical point not published by this chip
- acpitz/unlabelled 29.8 °C (board), critical point not published by this chip
- acpitz/unlabelled 29.8 °C (board), critical point not published by this chip
- acpitz/unlabelled 44.8 °C (board), critical point not published by this chip
- r8169_0_bf00:00/unlabelled 44.0 °C (unclassified), critical at 110.0 °C

## Accelerators

Read from `/sys/class/drm/card*/device`.

- card0 (amdgpu) 0% busy

## What MCF could not account for

- **Unrecognised sensor chip `r8169_0_bf00:00`.** Its readings are above; what is missing is which of them measures what.

