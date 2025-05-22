extern "C" {
void c_add(int a, int b, int *result) {
    *result = a + b;
}

void c_sub(int a, int b, int *result) {
    *result = a - b;
}
}