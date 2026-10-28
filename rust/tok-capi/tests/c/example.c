/* A small C program using the tok engine: formats a document, checks one with an
   error and prints the version. Built and run by tests/c_api.rs on Unix systems. */
#include <stdio.h>
#include <string.h>

#include "tok.h"

int main(void) {
    if (tok_abi_version() != TOK_ABI_VERSION) {
        fprintf(stderr, "unexpected ABI version %u\n", (unsigned)tok_abi_version());
        return 1;
    }
    printf("tok %s\n", tok_version());

    char *out = tok_fmt("tok 0.1\nstart   s\n", NULL);
    printf("%s\n", out);
    int ok = strncmp(out, "{\"ok\":", 6) == 0;
    tok_free(out);

    out = tok_bpmn("tok 0.1\nstart s\nflow s -> x\n", "{\"lang\": \"en\"}");
    printf("%s\n", out);
    ok = ok && strstr(out, "\"error\"") != NULL;
    tok_free(out);

    return ok ? 0 : 1;
}
