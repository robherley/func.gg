package main

import (
	"fmt"
	"os"
)

func init() {}

//go:wasmexport foo
func main() {
	for _, v := range os.Environ() {
		fmt.Println(v)
	}

	// fmt.Println("{")

	// x := 26
	// for i := range x {
	// 	fmt.Printf("  %q: %d", string('A'+i), i)
	// 	if i < x-1 {
	// 		fmt.Print(",")
	// 	}
	// 	fmt.Println()
	// 	time.Sleep(1 * time.Second)
	// }

	// fmt.Println("}")
}
