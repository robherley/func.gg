package main

import (
	"fmt"
	"time"
)

func main() {
	fmt.Println("{")

	x := 26
	for i := range x {
		fmt.Printf("  %q: %d", string('A'+i), i)
		if i < x-1 {
			fmt.Print(",")
		}
		fmt.Println()
		time.Sleep(1 * time.Second)
	}

	fmt.Println("}")
}
