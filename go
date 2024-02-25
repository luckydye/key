module key

go 1.21.7

require (
	github.com/charmbracelet/bubbles v0.18.0
	github.com/charmbracelet/bubbletea v0.25.0
	github.com/charmbracelet/log v0.3.1
	github.com/spf13/cobra v1.8.0
	github.com/tobischo/gokeepasslib/v3 v3.5.3
)

require (
	github.com/atotto/clipboard v0.1.4 // indirect
	github.com/aymanbagabas/go-osc52/v2 v2.0.1 // indirect
	github.com/charmbracelet/lipgloss v0.9.1 // indirect
	github.com/containerd/console v1.0.4-0.20230313162750-1ae8d489ac81 // indirect
	github.com/go-logfmt/logfmt v0.6.0 // indirect
	github.com/inconshreveable/mousetrap v1.1.0 // indirect
	github.com/lucasb-eyer/go-colorful v1.2.0 // indirect
	github.com/mattn/go-isatty v0.0.18 // indirect
	github.com/mattn/go-localereader v0.0.1 // indirect
	github.com/mattn/go-runewidth v0.0.15 // indirect
	github.com/muesli/ansi v0.0.0-20211018074035-2e021307bc4b // indirect
	github.com/muesli/cancelreader v0.2.2 // indirect
	github.com/muesli/reflow v0.3.0 // indirect
	github.com/muesli/termenv v0.15.2 // indirect
	github.com/rivo/uniseg v0.4.6 // indirect
	github.com/spf13/pflag v1.0.5 // indirect
	github.com/tobischo/argon2 v0.1.0 // indirect
	golang.org/x/crypto v0.18.0 // indirect
	golang.org/x/exp v0.0.0-20231006140011-7918f672742d // indirect
	golang.org/x/sync v0.1.0 // indirect
	golang.org/x/sys v0.17.0 // indirect
	golang.org/x/term v0.17.0 // indirect
	golang.org/x/text v0.14.0 // indirect
)
package main

import (
	"bufio"
	"fmt"
	"os"

	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
	logger "github.com/charmbracelet/log"
	"github.com/spf13/cobra"
	"github.com/tobischo/gokeepasslib/v3"
)

var log = logger.NewWithOptions(os.Stderr, logger.Options{
	ReportCaller:    true,
	ReportTimestamp: true,
})

func main() {
	var cmdList = &cobra.Command{
		Use:     "list",
		Short:   "List all entries in the database",
		Args:    cobra.MinimumNArgs(0),
		Aliases: []string{"ls"},
		Run: func(cmd *cobra.Command, args []string) {
			dbfile := os.Getenv("KEEPASSDB")
			keyfile := os.Getenv("KEEPASSDB_KEYFILE")

			reader := bufio.NewReader(os.Stdin)
			text, _ := reader.ReadByte()
			log.Info("io", "value", text)

			m := model{
				textInput: passwordPrompt(),
				err:       nil,
			}
			tm, _ := tea.NewProgram(&m, tea.WithOutput(os.Stderr)).Run()
			mm := tm.(model)

			log.Info("using dbfile", "path", dbfile)
			file, err := os.Open(dbfile)
			if err != nil {
				log.Error(err)
				return
			}

			pw := mm.textInput.Value()

			if pw == "" {
				log.Error("password is empty")
				return
			}

			log.Info("make credentials")

			db := gokeepasslib.NewDatabase()
			db.Credentials, err = gokeepasslib.NewPasswordAndKeyCredentials(pw, keyfile)
			if err != nil {
				log.Error(err)
				return
			}

			log.Info("decode database")
			err = gokeepasslib.NewDecoder(file).Decode(db)
			if err != nil {
				log.Error(err)
				return
			}

			log.Info("unlock entries")
			db.UnlockProtectedEntries()

			entry := db.Content.Root.Groups[0].Groups[0].Entries[0]
			fmt.Println(entry.GetTitle())
			fmt.Println(entry.GetPassword())
		},
	}

	var rootCmd = &cobra.Command{Use: "app"}

	rootCmd.AddCommand(cmdList)
	rootCmd.Execute()
}

// read password from terminal

type (
	errMsg error
)

type model struct {
	textInput textinput.Model
	err       error
}

func passwordPrompt() textinput.Model {
	t := textinput.New()
	t.Placeholder = "Password"
	t.EchoMode = textinput.EchoPassword
	t.EchoCharacter = '•'
	t.Focus()
	return t
}

func (m model) Init() tea.Cmd {
	return textinput.Blink
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmd tea.Cmd

	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.Type {
		case tea.KeyEnter, tea.KeyCtrlC, tea.KeyEsc:
			return m, tea.Quit
		}

	// We handle errors just like any other message
	case errMsg:
		m.err = msg
		return m, nil
	}

	m.textInput, cmd = m.textInput.Update(msg)
	return m, cmd
}

func (m model) View() string {
	return fmt.Sprintf(
		"%s\n\n%s",
		m.textInput.View(),
		"(esc to quit)",
	) + "\n"
}
