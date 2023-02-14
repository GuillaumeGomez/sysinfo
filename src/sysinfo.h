// Take a look at the license at the top of the repository in the LICENSE file.

#pragma once

#include <sys/types.h>
#include <stdbool.h>

typedef void* CSystem;
typedef const void* CProcess;
typedef const char* RString;

CSystem    *sysinfo_init();
void        sysinfo_destroy(CSystem system);

void        sysinfo_refresh_system(CSystem system);
void        sysinfo_refresh_memory(CSystem system);
void        sysinfo_refresh_cpu(CSystem system);
void        sysinfo_refresh_components(CSystem system);
void        sysinfo_refresh_all(CSystem system);
void        sysinfo_refresh_processes(CSystem system);
#ifdef __linux__
void        sysinfo_refresh_process(CSystem system, pid_t pid);
#endif
void        sysinfo_refresh_disks(CSystem system);
void        sysinfo_refresh_disk_list(CSystem system);

size_t      sysinfo_total_memory(CSystem system);
size_t      sysinfo_free_memory(CSystem system);
size_t      sysinfo_used_memory(CSystem system);
size_t      sysinfo_total_swap(CSystem system);
size_t      sysinfo_free_swap(CSystem system);
size_t      sysinfo_used_swap(CSystem system);

size_t      sysinfo_networks_received(CSystem system);
size_t      sysinfo_networks_transmitted(CSystem system);

void        sysinfo_cpus_usage(CSystem system, unsigned int *length, float **cpus);

size_t      sysinfo_processes(CSystem system, bool (*fn_pointer)(pid_t, CProcess, void*),
                              void *data);
#ifdef __linux__
size_t      sysinfo_process_tasks(CProcess process, bool (*fn_pointer)(pid_t, CProcess, void*),
                                  void *data);
#endif
CProcess    sysinfo_process_by_pid(CSystem system, pid_t pid);
pid_t       sysinfo_process_pid(CProcess process);
pid_t       sysinfo_process_parent_pid(CProcess process);
float       sysinfo_process_cpu_usage(CProcess process);
size_t      sysinfo_process_memory(CProcess process);
size_t      sysinfo_process_virtual_memory(CProcess process);
RString     sysinfo_process_executable_path(CProcess process);
RString     sysinfo_process_root_directory(CProcess process);
RString     sysinfo_process_current_directory(CProcess process);

void        sysinfo_rstring_free(RString str);
