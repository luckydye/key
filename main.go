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

			m := model{
				textInput: passwordPrompt(),
				err:       nil,
			}
			tm, _ := tea.NewProgram(&m, tea.WithOutput(os.Stderr)).Run()
			mm := tm.(model)

			// dbfile := os.Getenv("KEEPASSDB")
			// log.Info("using dbfile", "path", dbfile)
			// file, err := os.Open(dbfile)
			// if err != nil {
			// 	log.Error(err)
			// 	return
			// }

			stat, _ := os.Stdin.Stat()
			log.Info("stdin", "size", stat.Size())

			file := bufio.NewReader(os.Stdin)

			pw := mm.textInput.Value()

			if pw == "" {
				log.Error("password is empty")
				return
			}

			log.Info("make credentials")

			db := gokeepasslib.NewDatabase()

			keyfile := os.Getenv("KEEPASSDB_KEYFILE")

			creds, err := gokeepasslib.NewPasswordAndKeyCredentials(pw, keyfile)
			db.Credentials = creds

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

			// list all entries
			for _, group := range db.Content.Root.Groups {
				groupName := group.Name
				fmt.Printf("%s (%d)\n", groupName, len(group.Entries))
				for _, entry := range group.Entries {
					fmt.Println("  ", entry.GetTitle())
				}
			}
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
		"%s\n",
		m.textInput.View(),
	) + "\n"
}
