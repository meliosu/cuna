#include <stdint.h>

// Runtime

void request(uint32_t id, void *variable);
void submit(uint32_t id, void *variable);

// Datatype Typedefs

typedef int8_t i8;
typedef uint8_t u8;
typedef int16_t i16;
typedef uint16_t u16;
typedef int32_t i32;
typedef uint32_t u32;
typedef int64_t i64;
typedef uint64_t u64;
typedef void* pointer;

// General Type Definitions

typedef enum Type Type;
typedef enum Device Device;

enum Type {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    Pointer,
};

enum Device {
    Host,
    Cuda,
};

// Model Type Definitions

typedef struct Variable Variable;
typedef struct Operation Operation;
typedef struct Model Model;

struct Variable {
    uint32_t *producers;
    size_t n_producers;

    uint32_t *consumers;
    size_t n_consumers;

    Type type;
};

struct Operation {
    uint32_t *inputs;
    size_t n_inputs;

    uint32_t *outputs;
    size_t n_outputs;

    void (*function)();
    Device device;
};

struct Model {
    Variable *variables;
    size_t n_variables;

    Operation *operations;
    size_t n_operations;

    uint32_t *inputs;
    size_t n_inputs;

    uint32_t *outputs;
    size_t n_outputs;
};

// IDs

enum {
    ID_a,
    ID_b,
    ID_sum,
    ID_diff,
};

enum {
    ID_add,
    ID_sub,
};

// Helper arrays

uint32_t a_consumers[2] = { ID_add, ID_sub };
uint32_t b_consumers[2] = { ID_add, ID_sub };
uint32_t sum_producers[1] = { ID_add };
uint32_t diff_producers[1] = { ID_sub };

uint32_t add_inputs[2] = { ID_a, ID_b };
uint32_t add_outputs[1] = { ID_sum };
uint32_t sub_inputs[2] = { ID_a, ID_b };
uint32_t sub_outputs[1] = { ID_diff };

Variable variables[] = {
    { NULL, 0, &a_consumers, 2, I32 },
    { NULL, 0, &b_consumers, 2, I32 },
    { &sum_producers, 1, NULL, 0, I32 },
    { &diff_producers, 1, NULL, 0, I32 },
};

Operation operations[] = {
    { &add_inputs, 2, &add_outputs, 1, op_add, Host },
    { &sub_inputs, 2, &sub_outputs, 1, op_sub, Host },
};

uint32_t inputs[2] = {
    ID_a,
    ID_b,
};

uint32_t outputs[2] = {
    ID_sum,
    ID_diff,
};

// External Model

Model model = {
    .variables = &variables,
    .n_variables = 4,
    .operations = &operations,
    .n_operations = 2,
    .inputs = &inputs,
    .n_inputs = 2,
    .outputs = &outputs,
    .n_outputs = 2,
};

// Module Declarations

void c_add(int, int, int*);
void c_sub(int, int, int*);

// Operations

void op_add() {
    struct {
        int a;
        int b;
        int sum;
    } ctx;

    request(ID_a, &ctx.a);
    request(ID_b, &ctx.b);

    c_add(ctx.a, ctx.b, &ctx.sum);

    submit(ID_sum, &ctx.sum);
}

void op_sub() {
    struct {
        int a;
        int b;
        int diff;
    } ctx;

    request(ID_a, &ctx.a);
    request(ID_b, &ctx.b);

    c_sub(ctx.a, ctx.b, &ctx.diff);

    submit(ID_diff, &ctx.diff);
}