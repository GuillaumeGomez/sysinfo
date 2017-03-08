//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

#pragma once

#include <sys/types.h>
#include <stdbool.h>

typedef void* CSystem;
typedef const void* CProcess;

CSystem *sysinfo_init();
void     sysinfo_destroy(CSystem system);
void     sysinfo_refresh_system(CSystem system);
size_t   sysinfo_get_total_memory(CSystem system);
size_t   sysinfo_get_free_memory(CSystem system);
size_t   sysinfo_get_used_memory(CSystem system);
size_t   sysinfo_get_total_swap(CSystem system);
size_t   sysinfo_get_free_swap(CSystem system);
size_t   sysinfo_get_used_swap(CSystem system);
void     sysinfo_get_processors_usage(CSystem system, unsigned int *length, float **procs);
size_t   sysinfo_get_processes(CSystem system, bool (*fn_pointer)(pid_t, CProcess, void*),
                               void *data);
#ifdef __linux__
size_t   sysinfo_process_get_tasks(CProcess process, bool (*fn_pointer)(pid_t, CProcess, void*),
                                   void *data);
#endif
CProcess sysinfo_get_process_by_pid(CSystem system, pid_t pid);
pid_t    sysinfo_process_get_pid(CProcess process);
pid_t    sysinfo_process_get_parent_pid(CProcess process);
float    sysinfo_process_get_cpu_usage(CProcess process);
size_t   sysinfo_process_get_memory(CProcess process);
