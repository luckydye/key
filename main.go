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

var cmdList = &cobra.Command{
	Use:     "list",
	Short:   "List all entries in the database",
	Args:    cobra.MinimumNArgs(0),
	Aliases: []string{"ls"},
	Run: func(cmd *cobra.Command, args []string) {
		db := getDatabase()
		if db == nil {
			return
		}

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

var cmdGet = &cobra.Command{
	Use:     "get <name>",
	Short:   "Get an entry from the database",
	Args:    cobra.MinimumNArgs(1),
	Aliases: []string{"g"},
	Run: func(cmd *cobra.Command, args []string) {
		db := getDatabase()
		if db == nil {
			return
		}

		// get entry
		for _, group := range db.Content.Root.Groups {
			for _, entry := range group.Entries {
				if entry.GetTitle() == args[0] {
					fmt.Println(entry.GetPassword())
				}
			}
		}
	},
}

func readPassword() string {
	m := model{
		textInput: passwordPrompt(),
		err:       nil,
	}
	tm, _ := tea.NewProgram(&m, tea.WithOutput(os.Stderr)).Run()
	mm := tm.(model)
	return mm.textInput.Value()
}

func getDatabaseFile() *bufio.Reader {
	stat, _ := os.Stdin.Stat()

	if stat.Size() == 0 {
		dbfile := os.Getenv("KEEPASSDB")
		log.Debug("using dbfile", "path", dbfile)
		file, err := os.Open(dbfile)
		if err != nil {
			log.Error(err)
			return nil
		}
		return bufio.NewReader(file)
	}

	return bufio.NewReader(os.Stdin)
}

func getCredentials() *gokeepasslib.DBCredentials {
	keyfile := os.Getenv("KEEPASSDB_KEYFILE")

	pw := readPassword()

	if pw == "" {
		log.Error("password is empty")
		return nil
	}

	creds, err := gokeepasslib.NewPasswordAndKeyCredentials(pw, keyfile)
	if err != nil {
		log.Error(err)
		return nil
	}
	return creds
}

func getDatabase() *gokeepasslib.Database {
	file := getDatabaseFile()

	log.Debug("construct credentials")
	db := gokeepasslib.NewDatabase()
	creds := getCredentials()

	if creds == nil {
		return nil
	}

	db.Credentials = creds

	log.Debug("decode database")
	err := gokeepasslib.NewDecoder(file).Decode(db)
	if err != nil {
		log.Error(err)
		return nil
	}

	log.Debug("unlock entries")
	db.UnlockProtectedEntries()

	return db
}

func main() {
	var rootCmd = &cobra.Command{Use: "app"}
	rootCmd.AddCommand(cmdList, cmdGet)
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
	if m.err != nil {
		return ""
	}

	return fmt.Sprintf(
		"%s",
		m.textInput.View(),
	) + "\n"
}
