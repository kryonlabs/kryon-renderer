-- Simple test script to verify print works
print("Hello from Lua!")
print("Testing numbers:", 42, 3.14)
print("Testing booleans:", true, false)
print("Testing nil:", nil)
print("Testing multiple arguments:", "arg1", "arg2", "arg3")

-- Test that print works inside a function
function test_print()
    print("Print from inside function works!")
end

test_print()