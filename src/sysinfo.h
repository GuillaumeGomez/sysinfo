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
void        sysinfo_refresh_all(CSystem system);
void        sysinfo_refresh_processes(CSystem system);
#ifdef __linux__
void        sysinfo_refresh_process(CSystem system, pid_t pid);
#endif
void        sysinfo_refresh_disks(CSystem system);
void        sysinfo_refresh_disk_list(CSystem system);
size_t      sysinfo_get_total_memory(CSystem system);
size_t      sysinfo_get_free_memory(CSystem system);
size_t      sysinfo_get_used_memory(CSystem system);
size_t      sysinfo_get_total_swap(CSystem system);
size_t      sysinfo_get_free_swap(CSystem system);
size_t      sysinfo_get_used_swap(CSystem system);
size_t      sysinfo_get_network_income(CSystem system);
size_t      sysinfo_get_network_outcome(CSystem system);
void        sysinfo_get_cpus_usage(CSystem system, unsigned int *length, float **cpus);
size_t      sysinfo_get_processes(CSystem system, bool (*fn_pointer)(pid_t, CProcess, void*),
                                  void *data);
#ifdef __linux__
size_t      sysinfo_process_get_tasks(CProcess process, bool (*fn_pointer)(pid_t, CProcess, void*),
                                      void *data);
#endif
CProcess    sysinfo_get_process_by_pid(CSystem system, pid_t pid);
pid_t       sysinfo_process_get_pid(CProcess process);
pid_t       sysinfo_process_get_parent_pid(CProcess process);
float       sysinfo_process_get_cpu_usage(CProcess process);
size_t      sysinfo_process_get_memory(CProcess process);
RString     sysinfo_process_get_executable_path(CProcess process);
RString     sysinfo_process_get_root_directory(CProcess process);
RString     sysinfo_process_get_current_directory(CProcess process);
void        sysinfo_rstring_free(RString str);
