#include <stdio.h>

int main() {
    int counter = 0;

    printf("Starting debug test program...\n");

    // Breakpoint здесь (строка 8)
    counter = 42;

    printf("Counter value: %d\n", counter);

    // Еще один breakpoint (строка 12)
    counter = counter * 2;

    printf("Final counter: %d\n", counter);

    return 0;
}
