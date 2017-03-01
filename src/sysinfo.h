//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

#pragma once

void sysinfo_refresh_system();
size_t sysinfo_get_total_memory();
size_t sysinfo_get_free_memory();
size_t sysinfo_get_used_memory();
size_t sysinfo_get_total_swap();
size_t sysinfo_get_free_swap();
size_t sysinfo_get_used_swap();
void sysinfo_get_processors_usage(unsigned int *length, float **procs);
