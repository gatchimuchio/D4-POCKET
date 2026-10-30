#include "my_application.h"

void broker_process_channel_prepare_r2_e2e_environment();

int main(int argc, char** argv) {
  broker_process_channel_prepare_r2_e2e_environment();
  g_autoptr(MyApplication) app = my_application_new();
  return g_application_run(G_APPLICATION(app), argc, argv);
}
