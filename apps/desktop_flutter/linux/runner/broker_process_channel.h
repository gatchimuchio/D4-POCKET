#ifndef RUNNER_BROKER_PROCESS_CHANNEL_H_
#define RUNNER_BROKER_PROCESS_CHANNEL_H_

#include <flutter_linux/flutter_linux.h>
#include <gtk/gtk.h>

void broker_process_channel_register(FlPluginRegistry* registry,
                                     GtkWindow* owner_parent);

#endif  // RUNNER_BROKER_PROCESS_CHANNEL_H_
