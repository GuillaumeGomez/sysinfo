// Take a look at the license at the top of the repository in the LICENSE file.

#include <stdio.h>
#include <stdlib.h>
#include <sys/types.h>
#include <unistd.h>
#include <pthread.h>
#include "sysinfo.h"

void print_process(CProcess process) {
    RString exe = sysinfo_process_executable_path(process);
    printf(
        "process[%d]: parent: %d,\n"
        "             cpu_usage: %f,\n"
        "             memory: %ld,\n"
        "             virtual memory: %ld,\n"
        "             executable path: '%s'\n",
        sysinfo_process_pid(process), sysinfo_process_parent_pid(process), sysinfo_process_cpu_usage(process),
        sysinfo_process_memory(process), sysinfo_process_virtual_memory(process), exe);
    sysinfo_rstring_free(exe);
}

#ifdef __linux__
bool task_loop(pid_t /*pid*/, void* data) {
    (void)data;
    // printf("  ");
    // print_process(process);
    return true;
}

void* sleeping_func(void* data) {
    sleep(3);
    return data;
}
void check_tasks(CSystem system) {
    pthread_t thread;
    pthread_create(&thread, NULL, sleeping_func, NULL);
    sysinfo_refresh_processes(system);
    CProcess process = sysinfo_process_by_pid(system, getpid());
    printf("\n== Task(s) for current process: ==\n");
    print_process(process);
    printf("Got %ld task(s)\n", sysinfo_process_tasks(process, task_loop, NULL));
}
#else
void check_tasks(CSystem system) { (void)system; }
#endif

bool process_loop(pid_t /*pid*/, CProcess process, void* data) {
    unsigned int* i = (unsigned int*)data;

    print_process(process);
    *i += 1;
    return *i < 10;
}

int main() {
    CSystem system = sysinfo_init();
    CNetworks networks = sysinfo_networks_init();

    sysinfo_refresh_all(system);
    sysinfo_networks_refresh(networks);

    printf("os name:              %s\n", sysinfo_system_name());
    printf("os version:           %s\n", sysinfo_system_version());
    printf("kernel version:       %s\n", sysinfo_system_kernel_version());
    printf("long os version:      %s\n", sysinfo_system_long_version());
    printf("host name:            %s\n", sysinfo_system_host_name());
    printf("cpu vendor id:        %s\n", sysinfo_cpu_vendor_id(system));
    printf("cpu brand:            %s\n", sysinfo_cpu_brand(system));
    printf("cpu frequency:        %ld\n", sysinfo_cpu_frequency(system));
    printf("cpu cores:            %d\n", sysinfo_cpu_physical_cores());
    printf("total memory:         %zd\n", sysinfo_total_memory(system));
    printf("free memory:          %zd\n", sysinfo_free_memory(system));
    printf("used memory:          %zd\n", sysinfo_used_memory(system));
    printf("total swap:           %zd\n", sysinfo_total_swap(system));
    printf("free swap:            %zd\n", sysinfo_free_swap(system));
    printf("used swap:            %zd\n", sysinfo_used_swap(system));
    printf("networks received:    %zd\n", sysinfo_networks_received(networks));
    printf("networks transmitted: %zd\n", sysinfo_networks_transmitted(networks));
    unsigned int len = 0, i = 0;
    float* procs = NULL;
    sysinfo_cpus_usage(system, &len, &procs);
    while (i < len) {
        printf("CPU #%d usage: %f%%\n", i, procs[i]);
        i += 1;
    }
    free(procs);

    // processes part
    i = 0;
    printf("For a total of %ld processes.\n", sysinfo_processes(system, process_loop, &i));
    check_tasks(system);
    // we can now free the CSystem and the CNetworks objects.
    sysinfo_destroy(system);
    sysinfo_networks_destroy(networks);
    return 0;
}
