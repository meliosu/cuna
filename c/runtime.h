#include <stdint.h>
#include <stdlib.h>

typedef int8_t i8;
typedef uint8_t u8;
typedef int16_t i16;
typedef uint16_t u16;
typedef int32_t i32;
typedef uint32_t u32;
typedef int64_t i64;
typedef uint64_t u64;
typedef float f32;
typedef double f64;
typedef void *pointer;
typedef char *string;

typedef enum {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Pointer,
    String,
} Type;

typedef enum {
    Host,
    Cuda,
} Device;

typedef struct {
    uint32_t *producers;
    size_t n_producers;

    uint32_t *consumers;
    size_t n_consumers;

    Type type;
} Variable;

typedef struct {
    uint32_t *inputs;
    size_t n_inputs;

    uint32_t *outputs;
    size_t n_outputs;

    void (*function)();
    Device device;
} Operation;

typedef struct {
    Variable *variables;
    size_t n_variables;

    Operation *operations;
    size_t n_operations;

    uint32_t *inputs;
    size_t n_inputs;

    uint32_t *outputs;
    size_t n_outputs;
} Model;

#ifdef __cplusplus
extern "C" {
#endif

void request(uint32_t id, void *variable);
void submit(uint32_t id, void *variable);
void launch(Model *model);

#ifdef __cplusplus
}
#endif