#ifndef RUNNER_LINUX_TRAY_CONTROLLER_H_
#define RUNNER_LINUX_TRAY_CONTROLLER_H_

#include <flutter_linux/flutter_linux.h>
#include <gtk/gtk.h>

struct LinuxTrayController;

LinuxTrayController* linux_tray_controller_new(FlPluginRegistry* registry,
                                               GtkWindow* window,
                                               GApplication* application);
void linux_tray_controller_free(gpointer data);

#endif  // RUNNER_LINUX_TRAY_CONTROLLER_H_
