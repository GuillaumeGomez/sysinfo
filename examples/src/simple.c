// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

#include <stdio.h>
#include <stdlib.h>
#include "sysinfo.h"

bool process_loop(pid_t pid, CProcess process, void *data) {
    unsigned int *i = data;

    printf("process[%d]: parent: %d, cpu_usage: %f, memory: %ld\n",
           sysinfo_process_get_pid(process),
           sysinfo_process_get_parent_pid(process),
           sysinfo_process_get_cpu_usage(process),
           sysinfo_process_get_memory(process));
    *i += 1;
    if (*i >= 10) {
        return false;
    }
    return true;
}

int main() {
    CSystem system = sysinfo_init();
    sysinfo_refresh_system(system);
    printf("total memory: %ld\n", sysinfo_get_total_memory(system));
    printf("free memory:  %ld\n", sysinfo_get_free_memory(system));
    printf("used memory:  %ld\n", sysinfo_get_used_memory(system));
    printf("total swap:   %ld\n", sysinfo_get_total_swap(system));
    printf("free swap:    %ld\n", sysinfo_get_free_swap(system));
    printf("used swap:    %ld\n", sysinfo_get_used_swap(system));
    unsigned int len = 0, i = 0;
    float *procs = NULL;
    sysinfo_get_processors_usage(system, &len, &procs);
    while (i < len) {
        printf("Processor #%d usage: %f\n", i, procs[i]);
        i += 1;
    }
    free(procs);

    // processes part
    i = 0;
    printf("For a total of %d processes.\n", sysinfo_get_processes(system, process_loop, &i));
    // we can now free the CSystem object.
    sysinfo_destroy(system);
    return 0;
}
