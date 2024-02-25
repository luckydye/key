package main

import (
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
		Long:    `List all entries in the database`,
		Args:    cobra.MinimumNArgs(0),
		Aliases: []string{"ls"},
		Run: func(cmd *cobra.Command, args []string) {
			dbfile := os.Getenv("KEEPASSDB")
			keyfile := os.Getenv("KEEPASSDB_KEYFILE")

			log.Info("using dbfile", "path", dbfile)

			m := model{
				textInput: passwordPrompt(),
				err:       nil,
			}
			tm, _ := tea.NewProgram(&m, tea.WithOutput(os.Stderr)).Run()
			mm := tm.(model)

			file, err := os.Open(dbfile)
			if err != nil {
				log.Error(err)
				return
			}

			log.Info("make credentials")

			db := gokeepasslib.NewDatabase()
			db.Credentials, err = gokeepasslib.NewPasswordAndKeyCredentials(mm.textInput.Value(), keyfile)
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
