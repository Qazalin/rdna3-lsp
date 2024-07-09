local client = vim.lsp.start_client({
	name = "rdna3",
	cmd = { "/Users/qazal/code/rdna3/target/release/rdna3" },
})
print(client)
vim.lsp.buf_attach_client(0, client)
