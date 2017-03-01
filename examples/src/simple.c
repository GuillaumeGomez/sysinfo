// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

#include <stdio.h>
#include <stdlib.h>
#include "sysinfo.h"

int main() {
    sysinfo_refresh_system();
    printf("total memory: %ld\n", sysinfo_get_total_memory());
    printf("free memory:  %ld\n", sysinfo_get_free_memory());
    printf("used memory:  %ld\n", sysinfo_get_used_memory());
    printf("total swap:   %ld\n", sysinfo_get_total_swap());
    printf("free swap:    %ld\n", sysinfo_get_free_swap());
    printf("used swap:    %ld\n", sysinfo_get_used_swap());
    unsigned int len = 0, i = 0;
    float *procs = NULL;
    sysinfo_get_processors_usage(&len, &procs);
    while (i < len) {
        printf("Processor #%d usage: %f\n", i, procs[i]);
        i += 1;
    }
    free(procs);
    return 0;
}
