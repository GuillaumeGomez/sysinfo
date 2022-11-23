Struct containing a disk information.

## Linux

On linux, the [NFS](https://en.wikipedia.org/wiki/Network_File_System) file systems are ignored and the information of a mounted NFS can **no longer** be obtained via `System::refresh_disks_list`. This is due to the fact that I/O function `statvfs` used by `System::refresh_disks_list` is blocking and [may hang](https://github.com/GuillaumeGomez/sysinfo/pull/876) in some cases, such as calling `systemctl stop` to terminate the NFS service from the remote server.