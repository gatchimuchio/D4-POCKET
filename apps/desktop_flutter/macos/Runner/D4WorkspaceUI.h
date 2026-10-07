#pragma once
#include <stddef.h>
#include <stdint.h>
// 固定Rust UI ABI。OS scope実値、Broker資格、Approvalを返さない。
int32_t d4_workspace_select_and_write(const uint8_t * _Nonnull bytes, size_t count, int32_t pipe_fd);
void d4_workspace_release_last(void);
void d4_workspace_release_all(void);
