package main

import (
	"fmt"
	"time"

	"github.com/robherley/func.gg/examples/tinygo/gen/funcgg/runtime/responder"
)

//go:generate wit-bindgen-go generate -o gen/ ../../wit/

func main() {
	responder.SetStatus(200)
	responder.SetHeader("Content-Type", "application/json")

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
